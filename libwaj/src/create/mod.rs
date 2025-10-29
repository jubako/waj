mod creator;
mod entry;
mod entry_store_creator;
mod fs_adder;

use crate::error::CreatorError;
pub use creator::FsCreator;
pub use entry_store_creator::EntryStoreCreator;
pub use fs_adder::{FsAdder, Namer, StripPrefix};
use std::borrow::Cow;

pub enum EntryKind {
    Content(jbk::ContentAddress, mime_guess::Mime),
    Redirect(String),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> Result<Option<EntryKind>, CreatorError>;

    /// Under which name the entry will be stored
    fn name(&self) -> Cow<'_, str>;
}

pub type Void = Result<(), CreatorError>;
