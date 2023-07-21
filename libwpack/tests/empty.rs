use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn test_empty() {
    let wpack_file = tempfile::TempPath::from_path(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_empty.wpack"),
    );
    let creator = libwpack::create::FsCreator::new(
        &wpack_file,
        "".into(),
        "main_page".into(),
        libwpack::create::ConcatMode::OneFile,
        Arc::new(()),
        Rc::new(()),
    )
    .unwrap();
    assert!(creator.finalize(&wpack_file).is_err());
}
