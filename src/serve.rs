use crate::common::{AllProperties, Builder, Entry, Reader, RealBuilder};
use crate::Jim;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::Range;
use jubako as jbk;
use percent_encoding::{percent_decode, percent_encode, CONTROLS};
use std::net::ToSocketAddrs;
use std::path::Path;
use tiny_http::*;

fn url_variants(url: &str) -> Vec<&str> {
    let mut vec = vec![];
    vec.push(url);
    if let Some(idx) = url.find('?') {
        vec.push(&url[..idx])
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

struct PathBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for PathBuilder {
    type Entry = Vec<u8>;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &Reader) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = vec![];
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

type FullPathBuilder = (PathBuilder, PathBuilder);

pub struct Server {
    main_entry_path: String,
    jim: Jim,
}

impl Server {
    fn handle_get(&self, url: &str) -> jbk::Result<ResponseBox> {
        if url == "/" {
            let mut response = Response::empty(StatusCode(302));
            let location = percent_encode(self.main_entry_path.as_bytes(), CONTROLS);
            response.add_header(Header {
                field: "Location".parse().unwrap(),
                value: location.to_string().parse().unwrap(),
            });
            return Ok(response.boxed());
        };

        for url in url_variants(&url[1..]) {
            if let Ok(e) = self.jim.get_entry::<FullBuilder, _>(&url) {
                match e {
                    Entry::Content(e) => {
                        let reader = self.jim.get_reader(e.content_address)?;
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
        Ok(Response::empty(StatusCode(404)).boxed())
    }

    pub fn new<P: AsRef<Path>>(infile: P) -> jbk::Result<Self> {
        let jim = Jim::new(infile)?;
        // We have to found the main entry..

        let main_index = jim.get_index_for_name("jim_main")?;
        let properties = jim.create_properties(&main_index)?;
        let builder = RealBuilder::<FullPathBuilder>::new(&properties);
        let main_entry_path =
            String::from_utf8(match main_index.get_entry(&builder, 0.into())? {
                Entry::Content(p) => p,
                Entry::Redirect(p) => p,
            })?;

        Ok(Self {
            jim,
            main_entry_path,
        })
    }

    pub fn serve(&self, address: &str) -> jbk::Result<()> {
        let addr = address.to_socket_addrs().unwrap().next().unwrap();
        println!("Serving on address {addr}");
        let server = tiny_http::Server::http(addr).unwrap();

        loop {
            let request = match server.recv() {
                Err(e) => {
                    println!("error {e}");
                    break;
                }
                Ok(rq) => rq,
            };

            let url = percent_decode(request.url().as_bytes())
                .decode_utf8()
                .unwrap();

            let ret = match request.method() {
                Method::Get => self.handle_get(&url),
                _ => Err("Not a valid request".into()),
            };

            match ret {
                Err(e) => {
                    println!("Error : {e}");
                    request.respond(Response::empty(StatusCode(500))).unwrap();
                }
                Ok(response) => {
                    request.respond(response).unwrap();
                }
            }
        }
        Ok(())
    }
}
