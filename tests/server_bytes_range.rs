mod utils;

use ureq::http::header::{ACCEPT_RANGES, RANGE};

use core::ops::{Deref, Drop};
use std::path::Path;
use utils::*;

use rustest::{fixture, test};

#[fixture]
fn RefUrl(server: inner::Server) -> String {
    server.url("ref")
}

#[fixture(scope=global)]
fn BaseWajFile() -> TmpWaj {
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
}

pub struct TmpServer {
    child: std::process::Child,
    addr: String,
    _waj_file: BaseWajFile,
}

impl TmpServer {
    fn new(addr: String, waj_file: BaseWajFile) -> std::io::Result<Self> {
        let child = run!(spawn, "waj", "serve", waj_file.path(), &addr);
        Ok(Self {
            child,
            addr,
            _waj_file: waj_file,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}/{}", self.addr, path)
    }
}

impl Drop for TmpServer {
    fn drop(&mut self) {
        println!("Close server");
        self.child.kill().unwrap();
    }
}

mod inner {
    #[rustest::fixture(scope=global)]
    pub(super) fn Server(waj_file: super::BaseWajFile) -> super::TmpServer {
        let address = format!("localhost:{}", 5051);
        let server = super::TmpServer::new(address, waj_file).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        server
    }
}

#[test]
fn test_byte_range_support(_server: inner::Server, url: RefUrl) -> rustest::Result {
    let response = ureq::head(url.deref()).call()?;
    assert!(
        response.headers().contains_key(ACCEPT_RANGES),
        "Server does not support byte ranges"
    );
    Ok(())
}

#[test]
fn test_specific_byte_range(server: inner::Server) -> rustest::Result {
    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=0-99";
    let mut response = ureq::get(server.url("ref")).header(RANGE, range).call()?;

    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let expected_content = &full_content[0..=99];
    assert_eq!(
        response.body_mut().read_to_vec()?,
        expected_content,
        "Content does not match the requested range"
    );
    Ok(())
}

#[test]
fn test_exceeding_byte_range(server: inner::Server) -> rustest::Result {
    let range = "bytes=1000-2000";
    let response = ureq::get(server.url("ref"))
        .header(RANGE, range)
        .config()
        .http_status_as_error(false)
        .build()
        .call()?;

    assert_eq!(response.status(), 416, "Expected status code is 416");
    Ok(())
}

#[test]
fn test_multiple_byte_ranges(server: inner::Server) -> rustest::Result {
    let range = "bytes=0-49,51-99";
    let response = ureq::get(server.url("ref"))
        .header(RANGE, range)
        .config()
        .http_status_as_error(false)
        .build()
        .call()?;

    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.

    assert_eq!(response.status(), 416, "Expected status code is 416");
    Ok(())
}

#[test]
fn test_overlapping_byte_ranges(server: inner::Server) -> rustest::Result {
    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.
    let range = "bytes=0-49,40-99";
    let response = ureq::get(server.url("ref"))
        .header(RANGE, range)
        .config()
        .http_status_as_error(false)
        .build()
        .call()?;

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for partial content"
    );
    Ok(())
}

#[test]
fn test_reverse_byte_range(server: inner::Server) -> rustest::Result {
    let range = "bytes=99-0";
    let response = ureq::get(server.url("ref"))
        .header(RANGE, range)
        .config()
        .http_status_as_error(false)
        .build()
        .call()?;

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for reversed byte range"
    );
    Ok(())
}

#[test]
fn test_suffix_byte_range(server: inner::Server) -> rustest::Result {
    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=-100";
    let mut response = ureq::get(server.url("ref")).header(RANGE, range).call()?;

    assert_eq!(
        response.status(),
        206,
        "Expected status code 206 for partial content"
    );
    let expected_content = &full_content[full_content.len() - 100..];
    assert_eq!(
        response.body_mut().read_to_vec()?,
        expected_content,
        "Content does not match the requested suffix range"
    );
    Ok(())
}

#[rustest::main]
fn main() {}
