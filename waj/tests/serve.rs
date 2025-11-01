mod utils;

use rustest::{fixture, test, Result};
use std::path::Path;
use utils::*;

fn build_waj_file(directory: &Path, outfile: &Path) {
    cmd!(
        "waj",
        "create",
        "--outfile",
        outfile,
        "-C",
        directory.parent().unwrap(),
        "--strip-prefix",
        directory.file_name().unwrap(),
        directory.file_name().unwrap()
    )
    .check_output(Some(b""), Some(b""));
}

#[fixture(scope=global)]
fn BaseWajFile(source_dir: SharedTestDir) -> TmpWaj {
    let source_dir = source_dir.path();
    let tmp_waj_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_waj = tmp_waj_dir.path().join("test.waj");
    build_waj_file(source_dir, &tmp_waj);
    TmpWaj::new(tmp_waj_dir, tmp_waj)
}

#[test]
fn test_serve(source_dir: SharedTestDir, waj_file: BaseWajFile) -> Result {
    let addr = "localhost:5050";

    let mut command = cmd!("waj", "serve", waj_file.path(), "-a", &addr, "-vvv");

    let mut child = command.spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    tear_down!(CloseServer, || {
        child.kill().unwrap();
    });

    assert!(server_diff(Client::new(addr.into()), source_dir.path())?);
    Ok(())
}

#[test]
fn test_multi_serve_host(source_dir: SharedTestDir, waj_file: BaseWajFile) -> Result {
    let addr = "localhost:5052";
    let source_dir = source_dir.path();

    let tmp_waj_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_waj = tmp_waj_dir.path().join("sub_dir_a.waj");
    build_waj_file(&source_dir.join("sub_dir_a"), &tmp_waj);

    let mut command = cmd!(
        "waj",
        "serve",
        waj_file.path(),
        &tmp_waj,
        "-a",
        &addr,
        "-v",
        "--router",
        "host"
    );

    let mut child = command.spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    tear_down!(CloseServer, || {
        child.kill().unwrap();
    });

    assert!(server_diff(
        Client::new_with_host(addr.into(), "test.waj".into()),
        source_dir
    )?);
    assert!(server_diff(
        Client::new_with_host(addr.into(), "sub_dir_a.waj".into()),
        source_dir.join("sub_dir_a"),
    )?);

    let no_host_client = Client::new(addr.into());
    assert_eq!(
        no_host_client
            .get(&no_host_client.url("existing_file"))?
            .status(),
        400
    );
    Ok(())
}

#[test]
fn test_multi_serve_path(source_dir: SharedTestDir, waj_file: BaseWajFile) -> Result {
    let addr = "localhost:5053";
    let source_dir = source_dir.path();

    let tmp_waj_dir = tempfile::tempdir_in(Path::new(env!("CARGO_TARGET_TMPDIR")))
        .expect("Creating tmpdir should work");
    let tmp_waj = tmp_waj_dir.path().join("sub_dir_a.waj");
    build_waj_file(&source_dir.join("sub_dir_a"), &tmp_waj);

    let mut command = cmd!(
        "waj",
        "serve",
        waj_file.path(),
        &tmp_waj,
        "-a",
        &addr,
        "-v",
        "--router",
        "path"
    );

    let mut child = command.spawn()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    tear_down!(CloseServer, || {
        child.kill().unwrap();
    });

    assert!(server_diff(
        Client::new_with_subpath(addr.into(), "test.waj".into()),
        source_dir
    )?);
    assert!(server_diff(
        Client::new_with_subpath(addr.into(), "sub_dir_a.waj".into()),
        source_dir.join("sub_dir_a"),
    )?);

    let no_host_client = Client::new(addr.into());
    assert_eq!(
        no_host_client
            .get(&no_host_client.url("existing_file"))?
            .status(),
        400
    );
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
