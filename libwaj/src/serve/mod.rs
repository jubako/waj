use crate::error::WajError;
use crate::Waj;
use core::iter::Iterator;
use core::num::NonZeroUsize;
use log::error;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tiny_http::*;
mod handler;

use handler::RequestHandler;

pub struct Server {
    waj: Arc<Waj>,
    etag_value: String,
}

trait Router {
    fn route(&self, request: &Request) -> &RequestHandler;
}

impl Server {
    pub fn new<P: AsRef<Path>>(infile: P) -> Result<Self, WajError> {
        let waj = Arc::new(Waj::new(infile)?);
        let etag_value = "W/\"".to_owned() + &waj.uuid().to_string() + "\"";

        Ok(Self { waj, etag_value })
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

        let router = RequestHandler::new(
            Arc::clone(&self.waj),
            Arc::clone(&next_request_id),
            self.etag_value.clone(),
        );

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
                                let handler = router.route(&rq);
                                handler.handle(rq)
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
