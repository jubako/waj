mod utils;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_RANGES, RANGE};

use core::convert::From;
use core::ops::{Deref, Drop};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
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
    addr: String,
}

impl TmpServer {
    fn new(addr: String, waj_file: &Path) -> std::io::Result<Self> {
        let child = run!(spawn, "waj", "serve", &waj_file, &addr);
        Ok(Self { child, addr })
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

#[derive(Clone)]
struct ServerFixture(Arc<TmpServer>);

impl Deref for ServerFixture {
    type Target = TmpServer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TmpServer> for ServerFixture {
    fn from(value: TmpServer) -> Self {
        Self(Arc::new(value))
    }
}

fn tmp_server() -> TmpServer {
    let waj_file = BASE_WAJ_FILE.path();
    let address = format!("localhost:{}", 5051);
    let server = TmpServer::new(address, waj_file).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    server
}

fn test_byte_range_support(server: ServerFixture) -> Result {
    let client = Client::new();
    let response = client.head(server.url("ref")).send()?;
    assert!(
        response.headers().contains_key(ACCEPT_RANGES),
        "Server does not support byte ranges"
    );
    Ok(())
}

fn test_specific_byte_range(server: ServerFixture) -> Result {
    let client = Client::new();
    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=0-99";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

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
    Ok(())
}

fn test_exceeding_byte_range(server: ServerFixture) -> Result {
    let client = Client::new();
    let range = "bytes=1000-2000";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

    assert_eq!(response.status(), 416, "Expected status code is 416");
    Ok(())
}

fn test_multiple_byte_ranges(server: ServerFixture) -> Result {
    let client = Client::new();
    let range = "bytes=0-49,51-99";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.

    assert_eq!(response.status(), 416, "Expected status code is 416");
    Ok(())
}

fn test_overlapping_byte_ranges(server: ServerFixture) -> Result {
    let client = Client::new();
    // Current version of the server do not handle multipart response,
    // which is necessary to respond to multiple byte_ranges.
    // So we expect a 416, even if a server fully implementing the spec would return
    // a 206.
    let range = "bytes=0-49,40-99";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for partial content"
    );
    Ok(())
}

fn test_reverse_byte_range(server: ServerFixture) -> Result {
    let client = Client::new();
    let range = "bytes=99-0";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

    assert_eq!(
        response.status(),
        416,
        "Expected status code 416 for reversed byte range"
    );
    Ok(())
}

fn test_suffix_byte_range(server: ServerFixture) -> Result {
    let client = Client::new();
    let full_content: Vec<u8> = (0..=255).collect();
    let range = "bytes=-100";
    let response = client
        .get(server.url("ref"))
        .header(RANGE, range)
        .send()
        .unwrap();

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
    Ok(())
}

macro_rules! test {
    ($fixture: expr, $test: ident) => {{
        let fixture = $fixture.clone();
        paste::paste!{
            fn [<test_ $test>](fixture: ServerFixture) -> std::result::Result<(), libtest_mimic::Failed> {
                Ok($test(fixture)?)
            }
        }
        Trial::test(stringify!($test), move || paste::paste!([<test_ $test>](fixture)))
    }};
}

fn main() -> ExitCode {
    use libtest_mimic::{Arguments, Trial};
    let args = Arguments::from_args();
    let server: ServerFixture = tmp_server().into();
    let tests = vec![
        test!(server, test_byte_range_support),
        test!(server, test_specific_byte_range),
        test!(server, test_exceeding_byte_range),
        test!(server, test_multiple_byte_ranges),
        test!(server, test_overlapping_byte_ranges),
        test!(server, test_reverse_byte_range),
        test!(server, test_suffix_byte_range),
    ];
    let conclusion = libtest_mimic::run(&args, tests);
    println!("End of run");
    conclusion.exit_code()
}
