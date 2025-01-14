use std::borrow::Cow;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use jbk::creator::{BasicCreator, CachedContentAdder, ConcatMode};

use crate::error::CreatorError;

use super::{EntryKind, EntryStoreCreator, EntryTrait, FsAdder, Namer, Void};

struct Redirect {
    path: String,
    target: String,
}

impl EntryTrait for Redirect {
    fn kind(&self) -> Result<Option<EntryKind>, CreatorError> {
        Ok(Some(EntryKind::Redirect(self.target.clone())))
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed(&self.path)
    }
}

pub struct FsCreator {
    cached_content_creator: CachedContentAdder<BasicCreator>,
    entry_store_creator: Box<EntryStoreCreator>,
    namer: Box<dyn Namer>,
}

impl FsCreator {
    pub fn new<P: AsRef<Path>>(
        outfile: P,
        namer: Box<dyn Namer>,
        concat_mode: ConcatMode,
        progress: Arc<dyn jbk::creator::Progress>,
        cache_progress: Rc<dyn jbk::creator::CacheProgress>,
        compression: jbk::creator::Compression,
    ) -> jbk::creator::Result<Self> {
        let basic_creator = BasicCreator::new(
            outfile,
            concat_mode,
            crate::VENDOR_ID,
            compression,
            progress,
        )?;

        let entry_store_creator = Box::new(EntryStoreCreator::new(None));

        let cached_content_creator = CachedContentAdder::new(basic_creator, cache_progress);

        Ok(Self {
            cached_content_creator,
            entry_store_creator,
            namer,
        })
    }

    pub fn finalize(self, outfile: &Path) -> Void {
        Ok(self.cached_content_creator.into_inner().finalize(
            outfile,
            self.entry_store_creator,
            vec![],
        )?)
    }

    pub fn add_from_path(&mut self, path: &Path) -> Void {
        let mut fs_adder = FsAdder::new(&mut self.entry_store_creator, self.namer.as_ref());
        fs_adder.add_from_path(path, &mut self.cached_content_creator)
    }

    pub fn add_redirect(&mut self, path: &str, target: &str) -> Void {
        let redirect = Redirect {
            path: path.into(),
            target: target.into(),
        };
        self.entry_store_creator.add_entry(&redirect)
    }
}
