mod utils;

use format_bytes::format_bytes;
use std::{io::Read, path::Path};
use utils::*;

#[test]
fn test_crate_non_existant_input() -> Result {
    temp_waj!(waj_file);
    cmd!("waj", "create", "--outfile", &waj_file, "non_existant_dir").check_fail(
        b"",
        b"Error : Input non_existant_dir path doesn't exist or cannot be accessed\n",
    );
    assert!(!waj_file.exists());
    Ok(())
}

#[test]
fn test_crate_non_existant_output_directory() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_waj!(waj_file, "non_existant_directory/test.waj");
    cmd!(
        "waj",
        "create",
        "--outfile",
        &waj_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap()
    )
    .check_fail(
        b"",
        &format_bytes!(
            b"Error : Directory {} doesn't exist\n",
            waj_file.parent().unwrap().as_os_str().as_encoded_bytes()
        ),
    );
    assert!(!waj_file.exists());
    Ok(())
}

#[test]
fn test_crate_existant_output() -> Result {
    let tmp_source_dir = SHARED_TEST_DIR.path();
    temp_waj!(waj_file);
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&waj_file)?;
        f.write_all(b"Some dummy content")?;
    }

    // Try to write without --force
    cmd!(
        "waj",
        "create",
        "--outfile",
        &waj_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap()
    )
    .check_fail(
        b"",
        &format_bytes!(
            b"Error : File {} already exists. Use option --force to overwrite it.\n",
            waj_file.as_os_str().as_encoded_bytes()
        ),
    );
    assert_eq!(std::fs::read(&waj_file)?, b"Some dummy content");

    // Try to write without --force
    cmd!(
        "waj",
        "create",
        "--outfile",
        &waj_file,
        "-C",
        tmp_source_dir.parent().unwrap(),
        "--strip-prefix",
        tmp_source_dir.file_name().unwrap(),
        tmp_source_dir.file_name().unwrap(),
        "--force"
    )
    .check_output(Some(b""), Some(b""));
    {
        let mut f = std::fs::File::open(&waj_file)?;
        let mut buf = [0; 10];
        f.read_exact(&mut buf)?;
        assert_eq!(&buf, b"jbkC\x00\x00\x00\x00\x00\x02");
    }
    Ok(())
}
