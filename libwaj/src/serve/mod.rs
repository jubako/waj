use crate::error::WajError;
use crate::Waj;
use log::error;
use std::iter::Iterator;
use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tiny_http::*;
mod handler;

use handler::WajServer;

pub struct Server {
    router: WajServer,
}

trait Router {
    fn route(&self, request: &Request) -> &WajServer;
}

impl Server {
    pub fn new<P: AsRef<Path>>(infile: P) -> Result<Self, WajError> {
        let waj = Arc::new(Waj::new(infile)?);
        let etag_value = "W/\"".to_owned() + &waj.uuid().to_string() + "\"";

        Ok(Self {
            router: WajServer::new(waj, etag_value),
        })
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
                                handler.handle(rq, next_request_id.fetch_add(1, Ordering::Relaxed))
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
