use crate::common::{AllProperties, Builder, Entry, Reader};
use crate::Waj;
use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use log::{debug, error, info, trace};
use percent_encoding::{percent_decode, percent_encode, CONTROLS};
use std::borrow::Cow;
use std::net::ToSocketAddrs;
use std::ops::Deref;
use std::path::Path;
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

pub struct Server {
    waj: Waj,
}

impl Server {
    fn handle_get(&self, url: &str) -> jbk::Result<ResponseBox> {
        if url == "/" {
            let mut response = Response::empty(StatusCode(302));
            let location = percent_encode(&self.waj.main_entry_path, CONTROLS);
            response.add_header(Header {
                field: "Location".parse().unwrap(),
                value: location.to_string().parse().unwrap(),
            });
            return Ok(response.boxed());
        };

        for url in url_variants(&url[1..]) {
            if let Ok(e) = self.waj.get_entry::<FullBuilder, _>(&url.deref()) {
                trace!(" => {url}");
                match e {
                    Entry::Content(e) => {
                        let reader = self.waj.get_reader(e.content_address)?;
                        let mut response = Response::new(
                            StatusCode(200),
                            vec![],
                            reader.create_flux_all().to_owned(),
                            Some(reader.size().into_usize()),
                            None,
                        );
                        response.add_header(Header {
                            field: "Content-Type".parse().unwrap(),
                            value: String::from_utf8(e.mimetype)?.parse().unwrap(),
                        });
                        return Ok(response.boxed());
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
        info!("{url} not found");
        Ok(Response::empty(StatusCode(404)).boxed())
    }

    pub fn new<P: AsRef<Path>>(infile: P) -> jbk::Result<Self> {
        let waj = Waj::new(infile)?;

        Ok(Self { waj })
    }

    pub fn serve(&self, address: &str) -> jbk::Result<()> {
        let addr = address.to_socket_addrs().unwrap().next().unwrap();
        info!("Serving on address {addr}");
        let server = tiny_http::Server::http(addr).unwrap();

        loop {
            let request = match server.recv() {
                Err(e) => {
                    info!("error {e}");
                    break;
                }
                Ok(rq) => rq,
            };

            let url = percent_decode(request.url().as_bytes())
                .decode_utf8()
                .unwrap();

            let now = std::time::Instant::now();

            trace!("{}: {url}", request.method());

            let ret = match request.method() {
                Method::Get => self.handle_get(&url),
                _ => Err("Not a valid request".into()),
            };

            let elapsed_time = now.elapsed();

            match ret {
                Err(e) => {
                    error!("[{}µs] Error : {e}", elapsed_time.as_micros());
                    request.respond(Response::empty(StatusCode(500))).unwrap();
                }
                Ok(response) => {
                    trace!("[{}µs] Ok", elapsed_time.as_micros());
                    request.respond(response).unwrap();
                }
            }
        }
        Ok(())
    }
}
