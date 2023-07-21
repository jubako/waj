use jubako as jbk;

use crate::create::{EntryKind, EntryStoreCreator, EntryTrait, Void};
use mime_guess::mime;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

pub enum FsEntryKind {
    File(jbk::ContentAddress, mime::Mime),
    Link,
    Other,
}

pub trait Adder {
    fn add(&mut self, reader: jbk::Reader) -> jbk::Result<jbk::ContentAddress>;
}

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub path: PathBuf,
    pub name: OsString,
}

impl FsEntry {
    pub fn new_from_walk_entry(
        dir_entry: walkdir::DirEntry,
        name: OsString,
        adder: &mut dyn Adder,
    ) -> jbk::Result<Box<Self>> {
        let fs_path = dir_entry.path().to_path_buf();
        let attr = dir_entry.metadata().unwrap();
        let kind = if attr.is_file() {
            let reader: jbk::Reader = jbk::creator::FileSource::open(&fs_path)?.into();
            let mime_type = match mime_guess::from_path(&fs_path).first() {
                Some(m) => m,
                None => {
                    let mut buf = [0u8; 100];
                    let size = std::cmp::min(100, reader.size().into_usize());
                    reader
                        .create_flux_to(jbk::End::new_size(size))
                        .read_exact(&mut buf[..size])?;
                    (|| {
                        for window in buf[..size].windows(4) {
                            if window == b"html" {
                                return mime::TEXT_HTML;
                            }
                        }
                        mime::APPLICATION_OCTET_STREAM
                    })()
                }
            };
            let content_address = adder.add(reader)?;
            FsEntryKind::File(content_address, mime_type)
        } else if attr.is_symlink() {
            FsEntryKind::Link
        } else {
            FsEntryKind::Other
        };
        Ok(Box::new(Self {
            kind,
            path: fs_path,
            name,
        }))
    }
}

impl EntryTrait for FsEntry {
    fn kind(&self) -> jbk::Result<Option<EntryKind>> {
        Ok(match self.kind {
            FsEntryKind::File(content_address, ref mime) => {
                Some(EntryKind::Content(content_address, mime.clone()))
            }
            FsEntryKind::Link => Some(EntryKind::Redirect(fs::read_link(&self.path)?.into())),
            FsEntryKind::Other => None,
        })
    }
    fn name(&self) -> &OsStr {
        &self.name
    }
}

pub struct FsAdder<'a> {
    creator: &'a mut EntryStoreCreator,
    strip_prefix: &'a Path,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut EntryStoreCreator, strip_prefix: &'a Path) -> Self {
        Self {
            creator,
            strip_prefix,
        }
    }

    pub fn add_from_path<P, A>(&mut self, path: P, recurse: bool, adder: &mut A) -> Void
    where
        P: AsRef<std::path::Path>,
        A: Adder,
    {
        self.add_from_path_with_filter(path, recurse, |_e| true, adder)
    }

    pub fn add_from_path_with_filter<P, F, A>(
        &mut self,
        path: P,
        recurse: bool,
        filter: F,
        adder: &mut A,
    ) -> Void
    where
        P: AsRef<std::path::Path>,
        F: FnMut(&walkdir::DirEntry) -> bool,
        A: Adder,
    {
        let mut walker = walkdir::WalkDir::new(path);
        if !recurse {
            walker = walker.max_depth(0);
        }
        let walker = walker.into_iter();
        for entry in walker.filter_entry(filter) {
            let entry = entry.unwrap();
            let wpack_path = entry
                .path()
                .strip_prefix(self.strip_prefix)
                .unwrap()
                .as_os_str()
                .to_os_string();
            if wpack_path.is_empty() {
                continue;
            }
            let entry = FsEntry::new_from_walk_entry(entry, wpack_path, adder)?;
            self.creator.add_entry(entry.as_ref())?;
        }
        Ok(())
    }
}
