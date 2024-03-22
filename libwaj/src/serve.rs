use crate::common::{AllProperties, Builder, Entry, Reader};
use crate::Waj;
use ascii::IntoAsciiString;
use jbk::reader::builder::PropertyBuilderTrait;
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

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
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

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
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

    fn handle_get(&self, url: &str, with_content: bool) -> jbk::Result<ResponseBox> {
        for url in url_variants(url) {
            let url = url.strip_prefix('/').unwrap_or(&url);
            if let Ok(e) = self.waj.get_entry::<FullBuilder>(url) {
                trace!(" => {url}");
                match e {
                    Entry::Content(e) => {
                        let reader = self.waj.get_reader(e.content_address)?;
                        let mut response = if with_content {
                            Response::new(
                                StatusCode(200),
                                vec![],
                                reader.create_flux_all().to_owned(),
                                Some(reader.size().into_usize()),
                                None,
                            )
                            .boxed()
                        } else {
                            Response::empty(StatusCode(200)).boxed()
                        };
                        response.add_header(Header {
                            field: "Content-Type".parse().unwrap(),
                            value: String::from_utf8(e.mimetype)?.parse().unwrap(),
                        });
                        response.add_header(Header {
                            field: "Content-Length".parse().unwrap(),
                            value: reader.size().into_usize().to_string().parse().unwrap(),
                        });
                        return Ok(response);
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
        warn!("{url} not found");
        if let Ok(Entry::Content(e)) = self.waj.get_entry::<FullBuilder>("404.html") {
            let reader = self.waj.get_reader(e.content_address)?;
            let mut response = Response::new(
                StatusCode(404),
                vec![],
                reader.create_flux_all().to_owned(),
                Some(reader.size().into_usize()),
                None,
            );
            response.add_header(Header {
                field: "Content-Type".parse().unwrap(),
                value: String::from_utf8(e.mimetype)?.parse().unwrap(),
            });
            Ok(response.boxed())
        } else {
            Ok(Response::empty(StatusCode(404)).boxed())
        }
    }

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
            Ok(mut response) => {
                trace!("[{request_id} {}µs {url}] Ok", elapsed_time.as_micros());
                response.add_header(Header {
                    field: "Cache-Control".parse().unwrap(),
                    value: "max-age=86400, must-revalidate".parse().unwrap(),
                });
                response.add_header(Header {
                    field: "ETag".parse().unwrap(),
                    value: self.etag_value.clone().into_ascii_string().unwrap(),
                });
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
