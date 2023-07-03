use jubako as jbk;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use mime_guess::{Mime, mime};

#[test]
fn test_empty() {
    let wpack_file = tempfile::TempPath::from_path(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_empty.wpack"),
    );
    let creator = libwpack::create::Creator::new(
        &wpack_file,
        "main_page".into(),
        libwpack::create::ConcatMode::OneFile,
        Arc::new(()),
        Rc::new(()),
    )
    .unwrap();
    assert!(creator.finalize(&wpack_file).is_err());
}

struct SimpleEntry {
    name: OsString,
    content: String,
    mimetype: Mime,
}

impl libwpack::create::EntryTrait for SimpleEntry {
    fn name(&self) -> &OsStr {
        &self.name
    }

    fn kind(&self) -> jbk::Result<libwpack::create::EntryKind> {
        Ok(libwpack::create::EntryKind::Content(self.content.clone().into(), self.mimetype.clone()))
    }
}

#[test]
fn test_one_content() {
    let wpack_file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_one_content.wpack");
    //let wpack_file = tempfile::TempPath::from_path(wpack_file);

    let mut creator = libwpack::create::Creator::new(
        &wpack_file,
        "foo.txt".into(),
        libwpack::create::ConcatMode::TwoFiles,
        Arc::new(()),
        Rc::new(()),
    )
    .unwrap();
    let entry = SimpleEntry {
        name: "foo.txt".into(),
        content: "A Foo content".to_string(),
        mimetype: mime::TEXT_PLAIN,
    };
    creator.add_entry(entry).unwrap();
    creator.finalize(&wpack_file).unwrap();
    assert!(wpack_file.is_file());

    let wpack = libwpack::Wpack::new(&wpack_file).unwrap();
    assert_eq!(wpack.pack_count().into_u8(), 1);
    let index = wpack.get_index_for_name("wpack_entries").unwrap();
    assert!(!index.is_empty());

    let content_pack = wpack.get_pack(1.into()).unwrap();
    assert_eq!(content_pack.get_content_count().into_u32(), 1);
    let content_reader = content_pack.get_content(0.into()).unwrap();
    let mut content = vec![];
    content_reader
        .create_flux_all()
        .read_to_end(&mut content)
        .unwrap();
    assert_eq!(content, "A Foo content".as_bytes());
}

#[test]
fn test_one_content_concat() {
    let wpack_file = Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_one_content_concat.wpack");
    //let wpack_file = tempfile::TempPath::from_path(wpack_file);

    let mut creator = libwpack::create::Creator::new(
        &wpack_file,
        "foo.txt".into(),
        libwpack::create::ConcatMode::OneFile,
        Arc::new(()),
        Rc::new(()),
    )
    .unwrap();
    let entry = SimpleEntry {
        name: "foo.txt".into(),
        content: "A Foo content".to_string(),
        mimetype: mime::TEXT_PLAIN,
    };
    creator.add_entry(entry).unwrap();
    creator.finalize(&wpack_file).unwrap();
    assert!(wpack_file.is_file());

    let wpack = libwpack::Wpack::new(&wpack_file).unwrap();
    assert_eq!(wpack.pack_count().into_u8(), 1);
    let index = wpack.get_index_for_name("wpack_entries").unwrap();
    assert!(!index.is_empty());

    let content_pack = wpack.get_pack(1.into()).unwrap();
    assert_eq!(content_pack.get_content_count().into_u32(), 1);
    let content_reader = content_pack.get_content(0.into()).unwrap();
    let mut content = vec![];
    content_reader
        .create_flux_all()
        .read_to_end(&mut content)
        .unwrap();
    assert_eq!(content, "A Foo content".as_bytes());
}
