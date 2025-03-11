mod utils;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_RANGES, CONTENT_RANGE, RANGE};

use core::error::Error;
use core::ops::Drop;
use std::path::Path;
use std::sync::LazyLock;
use utils::*;

pub static BASE_WAJ_FILE: LazyLock<TmpWaj> = LazyLock::new(|| {
    let source_dir = (|| -> std::io::Result<_> {
        Ok(temp_tree!(0, {
            custom "ref" ((0..=255).collect::<Vec<u8>>())
        }))
    })()
    .unwrap();
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
});

struct TmpServer {
    child: std::process::Child,
    addr: &'static str,
}

impl TmpServer {
    fn new(addr: &'static str, waj_file: &Path) -> std::io::Result<Self> {
        let child = run!(spawn, "waj", "serve", &waj_file, &addr);
        Ok(Self { child, addr })
    }
}

impl Drop for TmpServer {
    fn drop(&mut self) {
        self.child.kill().unwrap();
    }
}

static COMMON_SERVER: LazyLock<TmpServer> = LazyLock::new(|| {
    let waj_file = BASE_WAJ_FILE.path();
    let server = TmpServer::new("localhost:5050", waj_file).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    server
});

#[test]
fn test_byte_range_support() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";
    let response = client.head(url).send()?;
    assert!(
        response.headers().contains_key(ACCEPT_RANGES),
        "Server does not support byte ranges"
    );
    println!("Server supports byte ranges.");
    Ok(())
}

#[test]
fn test_specific_byte_range() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";

    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=0-99";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let bytes = response.bytes().unwrap();
    let expected_content = &full_content[0..=99];
    assert_eq!(
        &bytes[..],
        expected_content,
        "Content does not match the requested range"
    );
    println!("Successfully retrieved byte range 0-99.");
    Ok(())
}

#[test]
fn test_exceeding_byte_range() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";

    let range = "bytes=1000-2000";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    if response.status() == 416 {
        println!("Requested range not satisfiable, as expected.");
    } else {
        panic!(
            "Unexpected status code: {}. Expected 416 for unsatisfiable range.",
            response.status()
        );
    };
    Ok(())
}

#[test]
fn test_multiple_byte_ranges() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";

    let range = "bytes=0-49,51-99";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.

    assert_eq!(response.status(), 416, "Expected status code is 416");
    Ok(())
    /*
    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let full_content: Vec<u8> = (0..=255).collect();
    let bytes = response.bytes().unwrap();
    let expected_content = [&full_content[0..=49], &full_content[50..=99]].concat();
    assert_eq!(
        &bytes[..],
        expected_content,
        "Content does not match the requested ranges"
    );
    println!("Successfully retrieved multiple byte ranges.");
    Ok(())
    */
}

#[test]
fn test_overlapping_byte_ranges() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";
    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.
    let range = "bytes=0-49,40-99";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for partial content"
    );
    Ok(())
    /*
    let full_content: Vec<u8> = (0..=255).collect();

    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let bytes = response.bytes().unwrap();
    let expected_content = &full_content[0..=99];
    assert_eq!(
        &bytes[..],
        expected_content,
        "Content does not match the requested ranges"
    );
    println!("Successfully retrieved overlapping byte ranges.");
    Ok(())
    */
}

#[test]
fn test_reverse_byte_range() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";
    let range = "bytes=99-0";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for reversed byte range"
    );
    Ok(())
}

#[test]
fn test_suffix_byte_range() -> Result {
    let client = Client::new();
    let url = "http://".to_owned() + COMMON_SERVER.addr + "/ref";
    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=-100";
    let response = client.get(url).header(RANGE, range).send().unwrap();

    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let bytes = response.bytes().unwrap();
    let expected_content = &full_content[full_content.len() - 100..];
    assert_eq!(
        &bytes[..],
        expected_content,
        "Content does not match the requested suffix range"
    );
    println!("Successfully retrieved byte range with suffix.");
    Ok(())
}
