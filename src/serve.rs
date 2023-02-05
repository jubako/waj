use crate::common::{Builder, Entry, EntryCompare, Jim, Schema};
use jbk::reader::schema::SchemaTrait;
use jubako as jbk;
use percent_encoding::percent_decode;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::rc::Rc;
use tiny_http::*;

fn url_variants(url: &str) -> Vec<&str> {
    let mut vec = vec![];
    vec.push(url);
    if let Some(idx) = url.find('?') {
        vec.push(&url[..idx])
    }
    vec
}

pub struct Server {
    main_entry: Entry,
    resolver: jbk::reader::Resolver,
    builder: Builder,
    index: jbk::reader::Index,
    jim: Jim,
}

impl Server {
    fn handle_get(&self, url: &str) -> jbk::Result<ResponseBox> {
        print!("--- Search for {url} ");
        if url == "/" {
            let mut response = Response::empty(StatusCode(302));
            response.add_header(Header {
                field: "Location".parse().unwrap(),
                value: self.main_entry.path()?.parse().unwrap(),
            });
            return Ok(response.boxed());
        };

        let finder: jbk::reader::Finder<Schema> = self.index.get_finder(&self.builder)?;

        for url in url_variants(&url[1..]) {
            let comparator = EntryCompare::new(&self.resolver, &self.builder, url.as_ref());
            let found = finder.find(&comparator)?;
            if let Some(idx) = found {
                println!(" Found entry {idx:?}");
                match finder.get_entry(idx)? {
                    Entry::Content(e) => {
                        //    println!("  content entry {:?}", e.path());
                        let reader = self.jim.get_reader(e.get_content_address())?;
                        let mut response = Response::new(
                            StatusCode(200),
                            vec![],
                            reader.create_stream_all(),
                            Some(reader.size().into_usize()),
                            None,
                        );
                        response.add_header(Header {
                            field: "Content-Type".parse().unwrap(),
                            value: e.get_mimetype().unwrap().parse().unwrap(),
                        });
                        return Ok(response.boxed());
                    }
                    Entry::Redirect(r) => {
                        /*       println!(
                            "  redirect entry {:?} to {:?}", r.path(),
                            r.get_target_link().unwrap()
                        );*/
                        let mut response = Response::empty(StatusCode(302));
                        response.add_header(Header {
                            field: "Location".parse().unwrap(),
                            value: r.get_target_link().unwrap().parse().unwrap(),
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
        let directory = jim.get_directory_pack();
        let value_storage = directory.create_value_storage();
        let entry_storage = directory.create_entry_storage();
        let index = directory.get_index_from_name("jim_entries")?;
        let builder = jim
            .schema
            .create_builder(index.get_store(&entry_storage)?)?;
        let resolver = jbk::reader::Resolver::new(Rc::clone(&value_storage));
        let main_finder: jbk::reader::Finder<Schema> = directory
            .get_index_from_name("jim_main")?
            .get_finder(&builder)?;
        let main_entry = main_finder.get_entry(0.into())?;

        Ok(Self {
            jim,
            builder,
            resolver,
            index,
            main_entry,
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

            if let Err(_e) = ret {
                request.respond(Response::empty(StatusCode(500))).unwrap();
            }
        }
        Ok(())
    }
}
