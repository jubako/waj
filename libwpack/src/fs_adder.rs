use jubako as jbk;

use crate::create::{EntryTrait, EntryKind, Creator};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{PathBuf};
use std::rc::Rc;

#[derive(PartialEq, Eq, Debug)]
pub enum FsEntryKind {
    Dir,
    File,
    Link,
    Other,
}

type Filter = Rc<dyn Fn(FsEntry) -> Option<FsEntry>>;

pub struct FsEntry {
    pub kind: FsEntryKind,
    pub path: PathBuf,
    pub name: OsString,
    filter: Filter,
}

impl FsEntry {
    fn new(path: PathBuf, name: OsString, filter: Filter) -> jbk::Result<Self> {
        let attr = fs::symlink_metadata(&path)?;
        Ok(if attr.is_dir() {
            Self {
                kind: FsEntryKind::Dir,
                path,
                name,
                filter,
            }
        } else if attr.is_file() {
            Self {
                kind: FsEntryKind::File,
                path,
                name,
                filter,
            }
        } else if attr.is_symlink() {
            Self {
                kind: FsEntryKind::Link,
                path,
                name,
                filter,
            }
        } else {
            Self {
                kind: FsEntryKind::Other,
                path,
                name,
                filter,
            }
        })
    }

    pub fn new_from_fs(
        dir_entry: fs::DirEntry,
        filter: Filter,
    ) -> jbk::Result<Self> {
        let path = dir_entry.path();
        let name = dir_entry.file_name();
        Ok(if let Ok(file_type) = dir_entry.file_type() {
            if file_type.is_dir() {
                Self {
                    kind: FsEntryKind::Dir,
                    path,
                    name,
                    filter,
                }
            } else if file_type.is_file() {
                Self {
                    kind: FsEntryKind::File,
                    path,
                    name,
                    filter,
                }
            } else if file_type.is_symlink() {
                Self {
                    kind: FsEntryKind::Link,
                    path,
                    name,
                    filter,
                }
            } else {
                Self {
                    kind: FsEntryKind::Other,
                    path,
                    name,
                    filter,
                }
            }
        } else {
            Self {
                kind: FsEntryKind::Other,
                path,
                name,
                filter,
            }
        })
    }
}

impl EntryTrait for FsEntry {
    fn kind(&self) -> jbk::Result<EntryKind> {
        Ok(match self.kind {
            FsEntryKind::File => {
                EntryKind::Content(jbk::creator::FileSource::open(&self.path)?.into())
            }
            FsEntryKind::Link => EntryKind::Redirect(fs::read_link(&self.path)?.into()),
            FsEntryKind::Dir => unreachable!(),
            FsEntryKind::Other => unreachable!(),
        })
    }
    fn name(&self) -> &OsStr {
        &self.name
    }
}

type Void = jbk::Result<()>;

pub struct FsAdder<'a> {
    creator: &'a mut Creator,
    strip_prefix: PathBuf,
}

impl<'a> FsAdder<'a> {
    pub fn new(creator: &'a mut Creator, strip_prefix: PathBuf) -> Self {
        Self {
            creator,
            strip_prefix,
        }
    }

    pub fn add_from_path<P: AsRef<std::path::Path>>(&mut self, path: P, recurse: bool) -> Void {
        self.add_from_path_with_filter(path, recurse, Rc::new(&Some))
    }

    pub fn add_from_path_with_filter<P>(&mut self, path: P, recurse: bool, filter: Filter) -> Void
    where
        P: AsRef<std::path::Path>,
    {
        let rel_path = path.as_ref().strip_prefix(&self.strip_prefix).unwrap();
        if rel_path.as_os_str().is_empty() {
            if recurse {
                for sub_entry in fs::read_dir(path)? {
                    let sub_entry = sub_entry?;
                    self.add_entry(FsEntry::new_from_fs(
                        sub_entry,
                        Rc::clone(&filter),
                    )?)?;
                }
            }
            Ok(())
        } else {
            self.add_entry(FsEntry::new(
                path.as_ref().to_path_buf(),
                path.as_ref().file_name().unwrap().to_os_string(),
                filter,
            )?)
        }
    }

    fn add_entry(&mut self, entry: FsEntry) -> Void {
        match entry.kind {
            FsEntryKind::File => self.creator.add_entry(entry),
            FsEntryKind::Link => self.creator.add_entry(entry),
            FsEntryKind::Dir => {
                let filter = Rc::clone(&entry.filter);
                for child in fs::read_dir(entry.path)? {
                    let child = child?;
                    self.add_entry(FsEntry::new_from_fs(child, Rc::clone(&filter))?)?;
                }
                Ok(())
            }
            FsEntryKind::Other => unreachable!()
        }
    }
}
