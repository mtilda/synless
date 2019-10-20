use editor::{make_json_lang, EditorCmd, MetaCommand, TextCmd, TextNavCmd, TreeCmd, TreeNavCmd};
use frontends::{Event, Frontend, Key, Terminal};
use language::Sort;
use pretty::{ColorTheme, DocLabel};

use crate::core_editor::Core;
use crate::error::ShellError;
use crate::keymaps::{FilterContext, FilteredKeymap, KeymapManager};
use crate::prog::{CallStack, DataStack, Prog, Value, Word};

use crate::data::example_keymaps;
use crate::data::example_pane_notation::make_example_pane_notation;
use crate::data::keyhint_lang::make_keyhint_lang;
use crate::data::message_lang::make_message_lang;

/// Demonstrate a basic interactive tree editor
pub struct ShellEditor {
    core: Core<'static>,
    frontend: Terminal,
    data_stack: DataStack<'static>,
    call_stack: CallStack<'static>,
    keymap_manager: KeymapManager<'static>,
}

impl ShellEditor {
    pub fn new() -> Result<Self, ShellError> {
        let core = Core::new(
            make_example_pane_notation(),
            make_keyhint_lang(),
            make_message_lang(),
            make_json_lang(),
        )?;
        let mut keymap_manager = KeymapManager::new();
        keymap_manager.register_mode("tree".into(), example_keymaps::make_tree_map());
        keymap_manager.register_mode("speed_bool".into(), example_keymaps::make_speed_bool_map());
        keymap_manager.register_menu(
            "node".into(),
            example_keymaps::make_node_map(
                core.language(core.lang_name_of(&DocLabel::ActiveDoc)?)?,
            ),
        );
        keymap_manager.replace_text_keymap(example_keymaps::make_text_map());

        let mut ed = ShellEditor {
            core,
            frontend: Terminal::new(ColorTheme::default_dark())?,
            data_stack: DataStack::new(),
            call_stack: CallStack::new(),
            keymap_manager,
        };

        // Set initial keymap
        ed.call(Word::Literal(Value::ModeName("tree".into())))?;
        ed.call(Word::PushMode)?;

        // ed.core.clear_messages()?;
        Ok(ed)
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        loop {
            if self.keymap_manager.has_active_menu() {
                self.handle_input()?;
            } else {
                if let Some(word) = self.call_stack.next() {
                    if let Err(err) = self.call(word) {
                        self.core.show_message(&format!("Error: {}", err))?;
                    }
                } else {
                    self.core.exec(MetaCommand::EndGroup)?;
                    self.handle_input()?;
                }
            }
        }
    }

    fn handle_input(&mut self) -> Result<(), ShellError> {
        let doc = self.core.active_doc()?;
        let tree_context = if doc.in_tree_mode() {
            Some(FilterContext {
                required_sort: doc.self_sort(),
                self_arity: doc.self_arity_type(),
                parent_arity: doc.parent_arity_type(),
            })
        } else {
            None
        };
        let keymap = self.keymap_manager.get_active_keymap(tree_context)?;

        self.update_key_hints(&keymap)?;
        self.core.redisplay(&mut self.frontend)?;
        match self.next_event(&keymap) {
            Ok(prog) => {
                self.call_stack.push(prog);
                self.keymap_manager.deactivate_menu();
                Ok(())
            }
            Err(ShellError::KeyboardInterrupt) => Err(ShellError::KeyboardInterrupt),
            Err(err) => Ok(self.core.show_message(&format!("Error: {}", err))?),
        }
    }

    fn update_key_hints(&mut self, keymap: &FilteredKeymap) -> Result<(), ShellError> {
        let lang_name = self.core.lang_name_of(&DocLabel::KeyHints)?;

        let mut dict_node = self.core.new_node("dict", lang_name)?;

        for (key, prog) in self.keymap_manager.hints(keymap) {
            let mut key_node = self.core.new_node("key", lang_name)?;
            key_node.inner().unwrap_text().text_mut(|t| {
                t.activate();
                t.set(key);
                t.inactivate();
            });

            let mut prog_node = self.core.new_node("prog", lang_name)?;
            prog_node.inner().unwrap_text().text_mut(|t| {
                t.activate();
                t.set(prog);
                t.inactivate();
            });

            let mut entry_node = self.core.new_node("entry", &lang_name)?;
            entry_node
                .inner()
                .unwrap_fixed()
                .replace_child(0, key_node)
                .unwrap();
            entry_node
                .inner()
                .unwrap_fixed()
                .replace_child(1, prog_node)
                .unwrap();
            let mut inner_dict = dict_node.inner().unwrap_flexible();
            inner_dict
                .insert_child(inner_dict.num_children(), entry_node)
                .unwrap();
        }
        self.core
            .exec_on(TreeCmd::Replace(dict_node), &DocLabel::KeyHints)?;

        let mut description_node = self
            .core
            .new_node_in_doc_lang("message", &DocLabel::KeymapName)?;
        description_node.inner().unwrap_text().text_mut(|t| {
            t.activate();
            t.set(keymap.name());
            t.inactivate();
        });
        self.core
            .exec_on(TreeCmd::Replace(description_node), &DocLabel::KeymapName)?;
        Ok(())
    }

    fn next_event(&mut self, keymap: &FilteredKeymap) -> Result<Prog<'static>, ShellError> {
        match self.frontend.next_event() {
            Some(Ok(Event::KeyEvent(Key::Ctrl('c')))) => Err(ShellError::KeyboardInterrupt),
            Some(Ok(Event::KeyEvent(key))) => self
                .keymap_manager
                .lookup(key, keymap)
                .ok_or_else(|| ShellError::UnknownKey(key)),
            Some(Err(err)) => Err(err.into()),
            _ => Err(ShellError::UnknownEvent),
        }
    }

    fn call(&mut self, word: Word<'static>) -> Result<(), ShellError> {
        Ok(match word {
            Word::Literal(value) => self.data_stack.push(value),
            Word::Apply => {
                let prog = self.data_stack.pop_quote()?;
                self.call_stack.push(prog);
            }
            Word::Swap => {
                self.data_stack.swap()?;
            }
            Word::Pop => {
                self.data_stack.pop()?;
            }
            Word::Echo => {
                let message = self.data_stack.pop_message()?;
                self.core.show_message(&message)?;
            }
            Word::NodeByName => {
                let (lang_name, construct_name) = self.data_stack.pop_lang_construct()?;
                let node = self.core.new_node(&construct_name, &lang_name)?;
                self.data_stack.push(Value::Tree(node));
            }
            Word::PushMode => {
                let name = self.data_stack.pop_mode_name()?;
                self.keymap_manager.push_mode(name)?;
            }
            Word::PopMode => {
                self.keymap_manager.pop_mode();
            }
            Word::ActivateMenu => {
                let name = self.data_stack.pop_menu_name()?;
                if self.keymap_manager.has_active_menu() {
                    // TODO decide how to handle this
                    panic!("Another menu is already active");
                }
                self.keymap_manager.activate_menu(name);
            }
            Word::ChildSort => {
                self.data_stack
                    .push(Value::Sort(self.core.active_doc()?.child_sort()));
            }
            Word::SelfSort => {
                self.data_stack
                    .push(Value::Sort(self.core.active_doc()?.self_sort()));
            }
            Word::SiblingSort => {
                self.data_stack
                    .push(Value::Sort(self.core.active_doc()?.sibling_sort()));
            }
            Word::AnySort => {
                self.data_stack.push(Value::Sort(Sort::any()));
            }
            Word::Remove => self.core.exec(TreeCmd::Remove)?,
            Word::Clear => self.core.exec(TreeCmd::Clear)?,
            Word::InsertHoleAfter => {
                self.core.exec(TreeCmd::InsertHoleAfter)?;
            }
            Word::InsertHoleBefore => {
                self.core.exec(TreeCmd::InsertHoleBefore)?;
            }
            Word::InsertHolePrepend => {
                self.core.exec(TreeCmd::InsertHolePrepend)?;
            }
            Word::InsertHolePostpend => {
                self.core.exec(TreeCmd::InsertHolePostpend)?;
            }
            Word::Replace => {
                let tree = self.data_stack.pop_tree()?;
                self.core.exec(TreeCmd::Replace(tree))?;
            }
            Word::Left => self.core.exec(TreeNavCmd::Left)?,
            Word::Right => self.core.exec(TreeNavCmd::Right)?,
            Word::Parent => self.core.exec(TreeNavCmd::Parent)?,
            Word::Child => {
                let index = self.data_stack.pop_usize()?;
                self.core.exec(TreeNavCmd::Child(index))?;
            }
            Word::Undo => self.core.exec(MetaCommand::Undo)?,
            Word::Redo => self.core.exec(MetaCommand::Redo)?,
            Word::Cut => self.core.exec(EditorCmd::Cut)?,
            Word::Copy => self.core.exec(EditorCmd::Copy)?,
            Word::PasteSwap => self.core.exec(EditorCmd::PasteSwap)?,
            Word::PopClipboard => self.core.exec(EditorCmd::PopClipboard)?,
            Word::GotoBookmark => {
                let name = self.data_stack.pop_char()?;
                let mark = self.core.get_bookmark(name)?;
                self.core.exec(TreeNavCmd::GotoBookmark(mark))?;
            }
            Word::SetBookmark => {
                let name = self.data_stack.pop_char()?;
                self.core.add_bookmark(name, &DocLabel::ActiveDoc)?;
            }
            Word::InsertChar => {
                let ch = self.data_stack.pop_char()?;
                self.core.exec(TextCmd::InsertChar(ch))?;
            }
            Word::DeleteCharBackward => self.core.exec(TextCmd::DeleteCharBackward)?,
            Word::DeleteCharForward => self.core.exec(TextCmd::DeleteCharForward)?,
            Word::TreeMode => self.core.exec(TextNavCmd::TreeMode)?,
            Word::TextLeft => self.core.exec(TextNavCmd::Left)?,
            Word::TextRight => self.core.exec(TextNavCmd::Right)?,
        })
    }
}
