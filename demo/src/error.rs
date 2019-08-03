use std::io;
use termion::event::Key;

use frontends::terminal;
use language::{ConstructName, LanguageName};
use pretty::PaneError;

#[derive(Debug)]
pub enum Error {
    UnknownKey(Key),
    UnknownKeymap(String),
    NoKeymap,
    UnknownEvent,
    KeyboardInterrupt,
    UnknownLang(LanguageName),
    UnknownConstruct {
        construct: ConstructName,
        lang: LanguageName,
    },
    ExpectedWord(String),
    EmptyStack,
    DocExec(String),
    Pane(PaneError<terminal::Error>),
    Io(io::Error),
    Term(terminal::Error),
}

impl From<PaneError<terminal::Error>> for Error {
    fn from(e: PaneError<terminal::Error>) -> Error {
        Error::Pane(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<terminal::Error> for Error {
    fn from(e: terminal::Error) -> Error {
        Error::Term(e)
    }
}
