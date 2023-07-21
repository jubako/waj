mod creator;
mod entry_store_creator;
mod fs_adder;

pub use creator::FsCreator;
pub use entry_store_creator::EntryStoreCreator;
pub use fs_adder::{Adder, FsAdder};
use std::ffi::OsStr;

use std::ffi::OsString;

pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

pub enum EntryKind {
    Content(jubako::ContentAddress, mime_guess::Mime),
    Redirect(OsString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jubako::Result<Option<EntryKind>>;

    /// Under which name the entry will be stored
    fn name(&self) -> &OsStr;
}

pub type Void = jubako::Result<()>;
