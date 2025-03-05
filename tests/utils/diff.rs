#![allow(dead_code)]

use core::{convert::From, unreachable};
use std::{
    cmp::Ordering,
    ffi::OsStr,
    fs::{read_dir, read_link, symlink_metadata, File, ReadDir},
    io::{self, BufReader, Read},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

struct ReadAsIter<R: Read>(BufReader<R>);

impl<R: Read> Iterator for ReadAsIter<R> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        let mut result: [u8; 1] = [0; 1];
        match self.0.read(&mut result).expect("read should succeed") {
            0 => None,
            1 => Some(result[0]),
            _ => unreachable!(),
        }
    }
}

impl<R: Read> From<R> for ReadAsIter<R> {
    fn from(value: R) -> Self {
        Self(BufReader::new(value))
    }
}

#[derive(Debug)]
pub enum TreeEntry {
    Dir(PathBuf),
    File(PathBuf),
    Link(PathBuf),
}

impl TreeEntry {
    fn new(p: &Path) -> io::Result<Self> {
        let metadata = symlink_metadata(p)?;
        if metadata.is_dir() {
            Ok(Self::Dir(p.to_path_buf()))
        } else if metadata.is_file() {
            Ok(Self::File(p.to_path_buf()))
        } else if metadata.is_symlink() {
            Ok(Self::Link(p.to_path_buf()))
        } else {
            unreachable!()
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            TreeEntry::Dir(p) => p,
            TreeEntry::File(p) => p,
            TreeEntry::Link(p) => p,
        }
    }

    pub fn file_name(&self) -> &OsStr {
        self.path().file_name().unwrap()
    }
}

impl PartialEq for TreeEntry {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TreeEntry::File(a), TreeEntry::File(b)) => {
                let file_a: ReadAsIter<_> = File::open(a).expect("Open should succeed").into();
                let file_b: ReadAsIter<_> = File::open(b).expect("Open should succeed").into();
                file_a.cmp(file_b) == Ordering::Equal
            }
            (TreeEntry::Link(a), TreeEntry::Link(b)) => {
                let target_a = read_link(a).expect("Read_link should succeed");
                let target_b = read_link(b).expect("Read_link should succeed");
                target_a.cmp(&target_b) == Ordering::Equal
            }
            _ => false,
        }
    }
}

struct EntryIterator(ReadDir);

impl EntryIterator {
    fn new(p: &Path) -> Self {
        Self(read_dir(p).unwrap())
    }
}

impl Iterator for EntryIterator {
    type Item = TreeEntry;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.0.next()?.expect("Iter dir should succeed");
        let file_type = next.file_type().unwrap();
        Some(if file_type.is_dir() {
            TreeEntry::Dir(next.path())
        } else if file_type.is_file() {
            TreeEntry::File(next.path())
        } else if file_type.is_symlink() {
            TreeEntry::Link(next.path())
        } else {
            unreachable!()
        })
    }
}

pub trait ContainEqual {
    fn contains(&self, p: &TreeEntry, root: &Path) -> bool;
}

impl ContainEqual for Vec<&Path> {
    fn contains(&self, e: &TreeEntry, root: &Path) -> bool {
        let p = match e {
            TreeEntry::File(p) => p,
            TreeEntry::Link(p) => p,
            TreeEntry::Dir(_) => unreachable!(),
        };
        let p = p.strip_prefix(root).unwrap();
        self.as_slice().contains(&p)
    }
}

pub fn list_diff(tested: &[u8], root: impl AsRef<Path>) -> std::io::Result<bool> {
    let test_list: Vec<_> = tested
        .split(|c| *c == b'\n')
        .map(|p| Path::new(OsStr::from_bytes(p)))
        .collect();
    let reference = TreeEntry::new(root.as_ref())?;
    diff_entry(&test_list, reference, root.as_ref())
}

struct Client {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl Client {
    fn new(base_url: String) -> Self {
        Self {
            base_url: String::from("http://") + &base_url + "/",
            client: reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
        }
    }
}

impl ContainEqual for Client {
    fn contains(&self, e: &TreeEntry, root: &Path) -> bool {
        let (abs_p, is_redirect) = match e {
            TreeEntry::File(p) => (p, false),
            TreeEntry::Link(p) => (p, true),
            TreeEntry::Dir(_) => unreachable!(),
        };
        let p = abs_p.strip_prefix(root).unwrap();

        let url = self.base_url.clone() + p.to_str().unwrap();
        let resp = self.client.get(&url).send().unwrap();
        if is_redirect && resp.status().is_redirection() {
            if let Some(location) = resp.headers().get(reqwest::header::LOCATION) {
                let target = read_link(abs_p).expect("Read_link should succeed");
                let target = target.to_str().unwrap();
                if location.to_str().unwrap().cmp(target) == Ordering::Equal {
                    true
                } else {
                    println!("Redirection not valid {:?} and {}", location, target);
                    false
                }
            } else {
                false
            }
        } else if !is_redirect && resp.status().is_success() {
            let file_content: ReadAsIter<_> =
                File::open(abs_p).expect("Open should succeed").into();
            file_content.cmp(resp.bytes().unwrap()) == Ordering::Equal
        } else {
            println!("No path {} ({}): {:?}", p.display(), url, resp);
            false
        }
    }
}

pub fn server_diff(url: &str, root: impl AsRef<Path>) -> std::io::Result<bool> {
    let client = Client::new(url.into());
    let reference = TreeEntry::new(root.as_ref())?;
    diff_entry(&client, reference, root.as_ref())
}

pub fn diff_entry(
    tested_content: &impl ContainEqual,
    reference: TreeEntry,
    root: &Path,
) -> std::io::Result<bool> {
    if let TreeEntry::Dir(path) = reference {
        for child in EntryIterator::new(&path) {
            if !diff_entry(tested_content, child, root)? {
                return Ok(false);
            }
        }
    } else {
        if !tested_content.contains(&reference, root) {
            return Ok(false);
        }
    }
    Ok(true)
}
