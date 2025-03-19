mod utils;

use std::{path::Path, sync::LazyLock};
use utils::*;

pub static BASE_WAJ_FILE: LazyLock<TmpWaj> = LazyLock::new(|| {
    let source_dir = SHARED_TEST_DIR.path();
    let tmp_waj_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_waj = tmp_waj_dir.path().join("test.waj");
    cmd!(
        "waj",
        "create",
        "--outfile",
        &tmp_waj,
        "-C",
        source_dir.parent().unwrap(),
        "--strip-prefix",
        source_dir.file_name().unwrap(),
        source_dir.file_name().unwrap()
    )
    .check_output(Some(b""), Some(b""));
    TmpWaj::new(tmp_waj_dir, tmp_waj)
});

#[test]
fn test_serve() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let waj_file = BASE_WAJ_FILE.path();
    let addr = "localhost:5050";

    let mut command = cmd!("waj", "serve", &waj_file, &addr);

    let mut child = command.spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    tear_down!(CloseServer, || {
        child.kill().unwrap();
    });

    assert!(server_diff(addr, tmp_source_dir,)?);
    Ok(())
}

#[test]
fn test_list() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    let waj_file = BASE_WAJ_FILE.path();

    let mut cmd = cmd!("waj", "list", &waj_file);
    let output = cmd.output()?.stdout;

    assert!(list_diff(&output, tmp_source_dir)?);
    Ok(())
}
