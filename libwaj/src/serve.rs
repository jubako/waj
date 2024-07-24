use crate::common::{AllProperties, Builder, Entry};
use crate::Waj;
use ascii::IntoAsciiString;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::{ByteRegion, ByteSlice};
use log::{debug, error, trace, warn};
use percent_encoding::{percent_decode, percent_encode, CONTROLS};
use std::borrow::Cow;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tiny_http::*;

fn url_variants(url: &str) -> Vec<Cow<str>> {
    let mut vec: Vec<Cow<str>> = vec![];
    vec.push(url.into());
    let query_string_idx = url.find('?');
    if let Some(idx) = query_string_idx {
        vec.push(url[..idx].into())
    }
    let end_idx = match query_string_idx {
        Some(idx) => idx,
        None => url.len(),
    };
    if url[..end_idx].ends_with('/') {
        let mut new_url = String::from(&url[..end_idx]);
        new_url.push_str("index.html");
        vec.push(new_url.into());
    }
    vec
}

struct ContentEntry {
    pub content_address: jbk::reader::ContentAddress,
    pub mimetype: Vec<u8>,
}

struct ContentBuilder {
    content_address_property: jbk::reader::builder::ContentProperty,
    content_mimetype_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for ContentBuilder {
    type Entry = ContentEntry;

    fn new(properties: &AllProperties) -> Self {
        Self {
            content_address_property: properties.content_address_property,
            content_mimetype_property: properties.content_mimetype_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let content_address = self.content_address_property.create(reader)?;
        let mut mimetype = Default::default();
        self.content_mimetype_property
            .create(reader)?
            .resolve_to_vec(&mut mimetype)?;
        Ok(ContentEntry {
            content_address,
            mimetype,
        })
    }
}

struct RedirectBuilder {
    target_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for RedirectBuilder {
    type Entry = Vec<u8>;

    fn new(properties: &AllProperties) -> Self {
        Self {
            target_property: properties.redirect_target_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let target_prop = self.target_property.create(reader)?;
        let mut target = vec![];
        target_prop.resolve_to_vec(&mut target)?;
        Ok(target)
    }
}

type FullBuilder = (ContentBuilder, RedirectBuilder);

// A internal server, local to one thread.
struct RequestHandler {
    waj: Arc<Waj>,
    next_request_id: Arc<AtomicUsize>,
    etag_value: String,
}

impl RequestHandler {
    fn new(waj: Arc<Waj>, next_request_id: Arc<AtomicUsize>, etag_value: String) -> Self {
        Self {
            waj,
            next_request_id,
            etag_value,
        }
    }

    fn build_response_from_read<R: std::io::Read + Send + 'static>(
        reader: R,
        size: Option<usize>,
        with_content: bool,
        status_code: u16,
    ) -> ResponseBox {
        if with_content {
            Response::new(StatusCode(status_code), vec![], reader, size, None).boxed()
        } else {
            Response::empty(StatusCode(status_code)).boxed()
        }
    }

    /// Build a response from a reader
    ///
    /// No tricky part.
    /// We set cache header as content will never change without waj change.
    fn build_response_from_bytes(
        &self,
        bytes: ByteRegion,
        with_content: bool,
        status_code: u16,
    ) -> ResponseBox {
        let mut response = Self::build_response_from_read(
            bytes.stream(),
            Some(bytes.size().into_usize()),
            with_content,
            status_code,
        );
        response.add_header(Header {
            field: "Content-Length".parse().unwrap(),
            value: bytes.size().into_usize().to_string().parse().unwrap(),
        });
        response.add_header(Header {
            field: "Cache-Control".parse().unwrap(),
            value: "max-age=86400, must-revalidate".parse().unwrap(),
        });
        response.add_header(Header {
            field: "ETag".parse().unwrap(),
            value: self.etag_value.clone().into_ascii_string().unwrap(),
        });
        response
    }

    /// Build a response from a content entry.
    ///
    /// The tricky part here is that we can have a found entry without a content
    /// (if the content pack is missing)
    ///
    /// If we have a content, simply build the response,
    /// If not, we have to generate a dummy content (and no cache, as it may change if server change)
    fn build_content_response(
        &self,
        bytes: jbk::reader::MayMissPack<ByteRegion>,
        with_content: bool,
        status_code: u16,
        mimetype: &str,
    ) -> jbk::Result<ResponseBox> {
        match bytes {
            jbk::reader::MayMissPack::MISSING(pack_info) => {
                let (msg, mimetype, status_code) = match mimetype {
                    "text/html" | "text/css" | "application/javascript" => {
                        let msg = format!(
                                            "<h1>Missing contentPack {}.</h1><p>Declared location is <pre>{}</pre></p><p>Found the pack and you are good !!</p>",
                                            pack_info.uuid,
                                            String::from_utf8_lossy(&pack_info.pack_location),
                                        );
                        (msg, "text/html", 503)
                    }
                    _ => {
                        let msg = format!(
                            r##"<?xml version="1.0" encoding="utf-8"?>
                                <svg width="800px" height="800px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="2.0">
                                    <path d="M10.5 15L13.5 12M13.5 15L10.5 12" stroke="#1C274C" stroke-width="1.5" stroke-linecap="round"/>
                                    <path d="M22 11.7979C22 9.16554 22 7.84935 21.2305 6.99383C21.1598 6.91514 21.0849 6.84024 21.0062 6.76946C20.1506 6 18.8345 6 16.2021 6H15.8284C14.6747 6 14.0979 6 13.5604 5.84678C13.2651 5.7626 12.9804 5.64471 12.7121 5.49543C12.2237 5.22367 11.8158 4.81578 11 4L10.4497 3.44975C10.1763 3.17633 10.0396 3.03961 9.89594 2.92051C9.27652 2.40704 8.51665 2.09229 7.71557 2.01738C7.52976 2 7.33642 2 6.94975 2C6.06722 2 5.62595 2 5.25839 2.06935C3.64031 2.37464 2.37464 3.64031 2.06935 5.25839C2 5.62595 2 6.06722 2 6.94975M21.9913 16C21.9554 18.4796 21.7715 19.8853 20.8284 20.8284C19.6569 22 17.7712 22 14 22H10C6.22876 22 4.34315 22 3.17157 20.8284C2 19.6569 2 17.7712 2 14V11" stroke="#1C274C" stroke-width="1.5" stroke-linecap="round"/>
                                    <text x="3" y="9" font-size="2" textLength="17" lengthAdjust="spacingAndGlyphs" fill="black">Missing pack</text>
                                    <text x="4" y="20" font-size="2" textLength="16" lengthAdjust="spacingAndGlyphs" fill="black">{0}</text>
                                </svg>"##,
                            pack_info.uuid
                        );
                        (msg, "image/svg+xml", 253)
                    }
                };

                let msg = std::io::Cursor::new(msg);
                let mut response =
                    Self::build_response_from_read(msg, None, with_content, status_code);
                response.add_header(Header {
                    field: "Content-Type".parse().unwrap(),
                    value: mimetype.parse().unwrap(),
                });
                response.add_header(Header {
                    field: "Cache-Control".parse().unwrap(),
                    value: "max-age=0, no-cache".parse().unwrap(),
                });
                Ok(response)
            }
            jbk::reader::MayMissPack::FOUND(bytes) => {
                let mut response = self.build_response_from_bytes(bytes, with_content, status_code);
                response.add_header(Header {
                    field: "Content-Type".parse().unwrap(),
                    value: mimetype.parse().unwrap(),
                });
                Ok(response)
            }
        }
    }

    /// Handle a get/head request for a url
    ///
    /// Mostly search for the entry, and generate corresponding response or 404.
    fn handle_get(&self, url: &str, with_content: bool) -> jbk::Result<ResponseBox> {
        // Search for entry... Using some variation around url (remove querystring, add index.html...)
        for url in url_variants(url) {
            let url = url.strip_prefix('/').unwrap_or(&url);
            if let Ok(e) = self.waj.get_entry::<FullBuilder>(url) {
                trace!(" => {url}");
                match e {
                    Entry::Content(e) => {
                        return self.build_content_response(
                            self.waj.get_bytes(e.content_address)?,
                            with_content,
                            200,
                            &String::from_utf8_lossy(&e.mimetype),
                        )
                    }
                    Entry::Redirect(r) => {
                        let mut response = Response::empty(StatusCode(302));
                        let location = format!("/{}", percent_encode(&r, CONTROLS));
                        response.add_header(Header {
                            field: "Location".parse().unwrap(),
                            value: location.parse().unwrap(),
                        });
                        return Ok(response.boxed());
                    }
                }
            }
        }

        // No entry found. Return 404. If we have one in the Waj use it, else return empty 404.
        warn!("{url} not found");
        if let Ok(Entry::Content(e)) = self.waj.get_entry::<FullBuilder>("404.html") {
            if let jbk::reader::MayMissPack::FOUND(bytes) = self.waj.get_bytes(e.content_address)? {
                let mut response = self.build_response_from_bytes(bytes, with_content, 404);
                response.add_header(Header {
                    field: "Content-Type".parse().unwrap(),
                    value: String::from_utf8_lossy(&e.mimetype).parse().unwrap(),
                });
                return Ok(response);
            }
        }
        Ok(Response::empty(StatusCode(404)).boxed())
    }

    /// Handle a request.
    ///
    /// This is mainly a wrapper around `handle_get` as we respond only to get/head request.
    /// The main work here is to:
    /// - Handle error (by returning a 500)
    /// - Handle get vs head (by requesting response without content)
    /// - Handle etag by requesting response without content if etag match and answering a 304.
    ///
    /// Cache header is not handle here as it depends of the response itself.
    fn handle(&self, request: Request) {
        trace!("Get req {request:?}");
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);

        let url = percent_decode(request.url().as_bytes())
            .decode_utf8()
            .unwrap();

        let now = std::time::Instant::now();

        debug!("[{request_id}] : {} {url}", request.method());

        let etag_match = if let Some(request_etag) = get_etag_from_headers(request.headers()) {
            request_etag == self.etag_value
        } else {
            false
        };

        let ret = match request.method() {
            Method::Get => self.handle_get(&url, !etag_match),
            Method::Head => self.handle_get(&url, false),
            _ => Err("Not a valid request".into()),
        };

        let elapsed_time = now.elapsed();

        match ret {
            Err(e) => {
                error!(
                    "[{request_id} {}µs {url}] Error : {e}",
                    elapsed_time.as_micros()
                );
                request.respond(Response::empty(StatusCode(500))).unwrap();
            }
            Ok(response) => {
                trace!("[{request_id} {}µs {url}] Ok", elapsed_time.as_micros());

                if etag_match {
                    request
                        .respond(response.with_status_code(StatusCode(304)))
                        .unwrap();
                } else {
                    request.respond(response).unwrap();
                }
            }
        }
    }
}

pub struct Server {
    waj: Arc<Waj>,
    etag_value: String,
}

fn get_etag_from_headers(headers: &[Header]) -> Option<String> {
    for header in headers {
        if header.field.equiv("if-none-match") {
            return Some(header.value.to_string());
        }
    }
    None
}
impl Server {
    pub fn new<P: AsRef<Path>>(infile: P) -> jbk::Result<Self> {
        let waj = Arc::new(Waj::new(infile)?);
        let etag_value = "W/\"".to_owned() + &waj.uuid().to_string() + "\"";

        Ok(Self { waj, etag_value })
    }

    pub fn serve(&self, address: &str) -> jbk::Result<()> {
        let addr = address.to_socket_addrs().unwrap().next().unwrap();
        let server = Arc::new(tiny_http::Server::http(addr).unwrap());
        let mut guards = Vec::with_capacity(4);
        let next_request_id = Arc::new(AtomicUsize::new(0));
        let quit_flag = Arc::new(AtomicBool::new(false));
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            signal_hook::flag::register_conditional_shutdown(signal, 1, Arc::clone(&quit_flag))?;
            signal_hook::flag::register(signal, Arc::clone(&quit_flag))?;
        }
        for _ in 0..4 {
            let server = server.clone();
            let handler = RequestHandler::new(
                Arc::clone(&self.waj),
                Arc::clone(&next_request_id),
                self.etag_value.clone(),
            );
            let quit_flag = Arc::clone(&quit_flag);

            let guard = std::thread::spawn(move || loop {
                if quit_flag.load(Ordering::Relaxed) {
                    break;
                }
                match server.recv_timeout(std::time::Duration::from_millis(500)) {
                    Err(e) => {
                        error!("error {e}");
                        break;
                    }
                    Ok(rq) => match rq {
                        Some(rq) => handler.handle(rq),
                        None => continue,
                    },
                };
            });

            guards.push(guard);
        }

        for guard in guards {
            guard.join().unwrap();
        }

        Ok(())
    }
}
