use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn test_empty() {
    let waj_file = tempfile::TempPath::from_path(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("test_empty.waj"),
    );
    let creator = libwaj::create::FsCreator::new(
        &waj_file,
        "".into(),
        "main_page".into(),
        libwaj::create::ConcatMode::OneFile,
        Arc::new(()),
        Rc::new(()),
    )
    .unwrap();
    assert!(creator.finalize(&waj_file).is_err());
}
