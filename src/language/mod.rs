mod forest;
mod language_set;
mod location;
mod node;
mod storage;
mod text;

use partial_pretty_printer as ppp;
use std::fmt;

pub use language_set::{
    AritySpec, ConstructSpec, GrammarSpec, LanguageSpec, NotationSetSpec, SortSpec,
};
pub use location::Location;
pub use node::{Node, NodeId};
pub use storage::Storage;

#[derive(thiserror::Error, fmt::Debug)]
pub enum LanguageError {
    // Grammar errors
    #[error("Duplicate key '{0}' used for both construct '{1}' and construct '{2}")]
    DuplicateKey(char, String, String),
    #[error("Duplicate name '{0}' used for two constructs")]
    DuplicateConstruct(String),
    #[error("Duplicate name '{0}' used for two sorts")]
    DuplicateSort(String),
    #[error("Duplicate name '{0}' used for both a construct and a sort")]
    DuplicateConstructAndSort(String),
    #[error("Name '{0}' is not a known construct or sort")]
    UndefinedConstructOrSort(String),
    // TODO: Check for cycles
    // #[error("Sort '{0}' refers to itself")]
    // InfiniteSort(String),

    // Notation sets
    #[error("The language '{0}' already has a notation set named '{1}'")]
    DuplicateNotationSet(String, String),
    #[error(
        "Notation set '{0}' gives a notation for '{1}', but there is no construct with that name"
    )]
    UndefinedNotation(String, String),
    #[error("Notation set '{0}' does not give a notation for construct '{1}'")]
    MissingNotation(String, String),
    #[error("Notation set '{0}' gives two notations for construct '{1}'")]
    DuplicateNotation(String, String),
    #[error("Invalid notation for construct '{1}' in notation set '{0}':\n{2}")]
    InvalidNotation(String, String, ppp::NotationError),

    // Languages
    #[error("Duplicate name '{0}' used for two languages")]
    DuplicateLanguage(String),
    #[error("Name '{0}' is not a known language")]
    UndefinedLanguage(String),
}
