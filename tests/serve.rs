mod utils;

use rustest::{fixture, test, Result};
use std::path::Path;
use utils::*;

#[fixture(scope=global)]
fn BaseWajFile(source_dir: SharedTestDir) -> TmpWaj {
    let source_dir = source_dir.path();
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
}

#[test]
fn test_serve(source_dir: SharedTestDir, waj_file: BaseWajFile) -> Result {
    let addr = "localhost:5050";

    let mut command = cmd!("waj", "serve", waj_file.path(), &addr);

    let mut child = command.spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    tear_down!(CloseServer, || {
        child.kill().unwrap();
    });

    assert!(server_diff(addr, source_dir.path())?);
    Ok(())
}

#[test]
fn test_list(source_dir: SharedTestDir, waj_file: BaseWajFile) -> Result {
    let mut cmd = cmd!("waj", "list", waj_file.path());
    let output = cmd.output()?.stdout;

    assert!(list_diff(&output, source_dir.path())?);
    Ok(())
}

#[rustest::main]
fn main() {}
