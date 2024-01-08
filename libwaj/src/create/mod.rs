mod creator;
mod entry;
mod entry_store_creator;
mod fs_adder;

pub use creator::FsCreator;
pub use entry_store_creator::EntryStoreCreator;
pub use fs_adder::{Adder, FsAdder};
use std::borrow::Cow;

pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

pub enum EntryKind {
    Content(jubako::ContentAddress, mime_guess::Mime),
    Redirect(String),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jubako::Result<Option<EntryKind>>;

    /// Under which name the entry will be stored
    fn name(&self) -> Cow<str>;
}

pub type Void = jubako::Result<()>;
