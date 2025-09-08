use log::error;
use percent_encoding::percent_decode_str;
use std::collections::HashMap;
use std::iter::Iterator;
use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tiny_http::*;
mod handler;

pub use handler::WajServer;

pub struct Server {
    router: Box<dyn Router>,
}

pub trait Router: Sync {
    fn route(&self, request: &Request) -> Option<(&WajServer, String)>;
}

pub struct HostRouter(HashMap<String, WajServer>);

impl HostRouter {
    pub fn new(map: HashMap<String, WajServer>) -> Self {
        Self(map)
    }
}

fn get_host_from_headers(headers: &[Header]) -> Option<String> {
    for header in headers {
        if header.field.equiv("host") {
            return Some(header.value.to_string());
        }
    }
    None
}

impl Router for HostRouter {
    fn route(&self, request: &Request) -> Option<(&WajServer, String)> {
        let host = get_host_from_headers(request.headers())?;
        Some((self.0.get(&host)?, request.url().into()))
    }
}

pub struct SubPathRouter(HashMap<String, WajServer>);

impl SubPathRouter {
    pub fn new(map: HashMap<String, WajServer>) -> Self {
        Self(map)
    }
}

impl Router for SubPathRouter {
    fn route(&self, request: &Request) -> Option<(&WajServer, String)> {
        let path = percent_decode_str(request.url()).decode_utf8().ok()?;

        let path = &path[1..];
        let (first_part, left_part) = path.split_once('/').unwrap_or((path, ""));
        Some((self.0.get(first_part)?, left_part.into()))
    }
}

impl Server {
    pub fn new(router: Box<dyn Router>) -> Self {
        Self { router }
    }

    pub fn serve(&self, address: &str, nb_threads: Option<NonZeroUsize>) -> jbk::Result<()> {
        let addr = address.to_socket_addrs().unwrap().next().unwrap();
        let server = Arc::new(tiny_http::Server::http(addr).unwrap());
        let next_request_id = Arc::new(AtomicUsize::new(0));
        let quit_flag = Arc::new(AtomicBool::new(false));
        for signal in [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM] {
            signal_hook::flag::register_conditional_shutdown(signal, 1, Arc::clone(&quit_flag))?;
            signal_hook::flag::register(signal, Arc::clone(&quit_flag))?;
        }
        let nb_threads = if let Some(t) = nb_threads {
            t
        } else {
            std::thread::available_parallelism()?
        };

        std::thread::scope(|s| {
            for _ in 0..nb_threads.into() {
                s.spawn(|| loop {
                    if quit_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    match server.recv_timeout(std::time::Duration::from_millis(500)) {
                        Err(e) => {
                            error!("error {e}");
                            break;
                        }
                        Ok(rq) => match rq {
                            Some(rq) => {
                                let handler = self.router.route(&rq);
                                if let Some((handler, path)) = handler {
                                    handler.handle(
                                        rq,
                                        path.as_ref(),
                                        next_request_id.fetch_add(1, Ordering::Relaxed),
                                    )
                                } else {
                                    rq.respond(Response::empty(400)).unwrap()
                                }
                            }
                            None => continue,
                        },
                    };
                });
            }
        });

        Ok(())
    }
}
