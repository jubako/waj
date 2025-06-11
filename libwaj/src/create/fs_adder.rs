use crate::create::{EntryKind, EntryStoreCreator, EntryTrait, Void};
use crate::error::CreatorError;
use core::option::Option::None;
use jbk::creator::{CompHint, ContentAdder, InputReader};
use mime_guess::mime;
use std::borrow::Cow;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub enum FsEntryKind {
    File(jbk::ContentAddress, mime::Mime),
    Link,
    Other,
}

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub path: PathBuf,
    pub name: String,
}

impl FsEntry {
    pub fn new_from_walk_entry(
        dir_entry: walkdir::DirEntry,
        name: String,
        adder: &mut impl ContentAdder,
    ) -> Result<Box<Self>, CreatorError> {
        let fs_path = dir_entry.path().to_path_buf();
        let attr = dir_entry.metadata().unwrap();
        let kind = if attr.is_file() {
            let mut reader = jbk::creator::InputFile::open(&fs_path)?;
            let mime_type = match mime_guess::from_path(&fs_path).first() {
                Some(m) => m,
                None => {
                    let mut buf = [0u8; 100];
                    let size = std::cmp::min(100, reader.size().into_u64() as usize);
                    reader.read_exact(&mut buf[..size])?;
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
            reader.seek(SeekFrom::Start(0))?;
            let content_address = adder.add_content(Box::new(reader), CompHint::Detect)?;
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
    fn kind(&self) -> Result<Option<EntryKind>, CreatorError> {
        Ok(match self.kind {
            FsEntryKind::File(content_address, ref mime) => {
                Some(EntryKind::Content(content_address, mime.clone()))
            }
            FsEntryKind::Link => {
                let path = &self.path;
                let target = fs::read_link(path)?;
                let abs_target = path.parent().unwrap().join(&target);
                if abs_target.is_dir() {
                    None
                } else {
                    Some(EntryKind::Redirect(
                        target
                            .to_str()
                            .unwrap_or_else(|| panic!("{path:?} must be a utf8"))
                            .to_owned(),
                    ))
                }
            }
            FsEntryKind::Other => None,
        })
    }
    fn name(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }
}

pub trait Namer {
    fn rename(&self, path: &Path) -> String;
}

pub struct StripPrefix {
    prefix: PathBuf,
}

impl StripPrefix {
    pub fn new(prefix: PathBuf) -> Self {
        Self { prefix }
    }
}

impl Namer for StripPrefix {
    fn rename(&self, path: &Path) -> String {
        path.strip_prefix(&self.prefix)
            .unwrap()
            .to_str()
            .unwrap_or_else(|| panic!("{path:?} must be a utf8"))
            .to_owned()
    }
}

pub struct FsAdder<'a> {
    creator: &'a mut EntryStoreCreator,
    namer: &'a dyn Namer,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut EntryStoreCreator, namer: &'a dyn Namer) -> Self {
        Self { creator, namer }
    }

    pub fn add_from_path<P>(&mut self, path: P, adder: &mut impl ContentAdder) -> Void
    where
        P: AsRef<std::path::Path>,
    {
        self.add_from_path_with_filter(path, |_e| true, adder)
    }

    pub fn add_from_path_with_filter<P, F>(
        &mut self,
        path: P,
        filter: F,
        adder: &mut impl ContentAdder,
    ) -> Void
    where
        P: AsRef<std::path::Path>,
        F: FnMut(&walkdir::DirEntry) -> bool,
    {
        let walker = walkdir::WalkDir::new(path);
        let walker = walker.into_iter();
        for entry in walker.filter_entry(filter) {
            let entry = entry.unwrap();
            let entry_path = entry.path();
            let waj_path = self.namer.rename(entry_path);
            if waj_path.is_empty() {
                continue;
            }
            let entry = FsEntry::new_from_walk_entry(entry, waj_path, adder)?;
            self.creator.add_entry(entry.as_ref())?;
        }
        Ok(())
    }
}
