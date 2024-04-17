use std::cell::RefCell;
use std::rc::Rc;
use synless::{log, ColorTheme, Log, Runtime, Settings, SynlessBug, Terminal};

// TODO: Make this work if you start in a different cwd
const BASE_MODULE_PATH: &str = "scripts/base_module.rhai";
const INTERNALS_MODULE_PATH: &str = "scripts/internals_module.rhai";
const INIT_PATH: &str = "scripts/init.rhai";
const MAIN_PATH: &str = "scripts/main.rhai";

fn make_engine() -> rhai::Engine {
    let mut engine = rhai::Engine::new();
    engine.set_fail_on_invalid_map_property(true);
    engine.on_print(|msg| log!(Info, "{msg}"));
    engine.on_debug(|msg, src, pos| {
        let src = src.unwrap_or("unknown");
        log!(Debug, "{src} @ {pos:?} > {msg}");
    });

    engine.build_type::<synless::KeyProg>();
    engine.build_type::<synless::SynlessError>();

    println!("Signatures:");
    engine
        .gen_fn_signatures(false)
        .into_iter()
        .for_each(|func| println!("  {func}"));
    println!();

    engine
}

fn make_runtime() -> Rc<RefCell<Runtime<Terminal>>> {
    let settings = Settings::default();
    let terminal =
        Terminal::new(ColorTheme::default_dark()).bug_msg("Failed to construct terminal frontend");
    let runtime = Runtime::new(settings, terminal);
    Rc::new(RefCell::new(runtime))
}

fn run() -> Result<(), Box<rhai::EvalAltResult>> {
    let mut engine = make_engine();

    // Load internals_module.rhai
    let mut internals_mod = {
        let internals_ast = engine.compile_file(INTERNALS_MODULE_PATH.into())?;
        rhai::Module::eval_ast_as_new(rhai::Scope::new(), &internals_ast, &engine)?
    };

    // Load base_module.rhai
    let mut base_mod = {
        let base_ast = engine.compile_file(BASE_MODULE_PATH.into())?;
        rhai::Module::eval_ast_as_new(rhai::Scope::new(), &base_ast, &engine)?
    };

    // Register runtime methods into internals_module and base_module
    let runtime = make_runtime();
    Runtime::register_internal_methods(runtime.clone(), &mut internals_mod);
    engine.register_static_module("synless_internals", internals_mod.into());
    Runtime::register_external_methods(runtime, &mut base_mod);
    engine.register_static_module("s", base_mod.into());

    // Can't set this before modules are registered, as they reference each other
    engine.set_strict_variables(true);

    // Load init.rhai
    let init_ast = engine.compile_file(INIT_PATH.into())?;
    engine.run_ast(&init_ast)?;

    // Load main.rhai
    let main_ast = engine.compile_file(MAIN_PATH.into())?;
    engine.run_ast(&main_ast)?;

    Ok(())
}

fn main() {
    log!(Info, "Synless is starting");
    if let Err(err) = run() {
        log!(Error, "{}", err);
    }
    println!("{}", Log::to_string());
}
