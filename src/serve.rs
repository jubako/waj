use crate::common::{Entry, EntryCompare, Jim, Schema};
use jbk::reader::schema::SchemaTrait;
use jubako as jbk;
use percent_encoding::percent_decode;
use std::net::SocketAddr;
use std::path::Path;
use std::rc::Rc;
use tiny_http::*;

pub fn serve<P: AsRef<Path>>(infile: P, address: &str, port: u16) -> jbk::Result<()> {
    let jim = Jim::new(infile)?;
    let directory = jim.get_directory_pack();
    let value_storage = directory.create_value_storage();
    let entry_storage = directory.create_entry_storage();
    let index = directory.get_index_from_name("entries")?;
    let builder = jim
        .schema
        .create_builder(index.get_store(&entry_storage)?)?;
    let resolver = jbk::reader::Resolver::new(Rc::clone(&value_storage));
    let finder: jbk::reader::Finder<Schema> = index.get_finder(&builder)?;

    let addr = SocketAddr::new(address.parse().unwrap(), port);
    println!("Serving on address {}", addr);
    let server = tiny_http::Server::http(addr).unwrap();

    loop {
        let request = match server.recv() {
            Err(e) => {
                println!("error {}", e);
                break;
            }
            Ok(rq) => rq,
        };

        if request.method() != &Method::Get {
            continue;
        }

        let url = percent_decode(request.url().as_bytes())
            .decode_utf8()
            .unwrap();
        println!("--- Search for {}", url);
        let comparator = EntryCompare::new(&resolver, &builder, url[1..].as_ref());
        let found = finder.find(&comparator)?;
        match found {
            None => {
                println!("No result => 404");
                request.respond(Response::empty(StatusCode(404)))
            }
            Some(idx) => match finder.get_entry(idx)? {
                Entry::Content(e) => {
                    println!("  Found content entry {:?}", idx);
                    let reader = jim.get_reader(e.get_content_address())?;
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
                    request.respond(response)
                }
                Entry::Redirect(r) => {
                    println!(
                        "  Found redirect entry {:?} to {:?}",
                        idx,
                        r.get_target_link().unwrap()
                    );
                    let mut response = Response::empty(StatusCode(302));
                    response.add_header(Header {
                        field: "Location".parse().unwrap(),
                        value: r.get_target_link().unwrap().parse().unwrap(),
                    });
                    request.respond(response)
                }
            },
        }
        .unwrap();
    }
    Ok(())
}
