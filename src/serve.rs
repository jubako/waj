use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{iter::Iterator, num::NonZeroUsize};

/// Serve the waj archive on the web.
#[derive(Parser)]
pub struct Options {
    /// Archive to serve
    #[arg(value_parser)]
    infiles: Vec<PathBuf>,

    /// On which address serve the archive.
    #[arg(short, long, value_parser, default_value = "localhost:1234")]
    address: String,

    /// Number of threads to use to answer request
    #[arg(short, long, value_parser)]
    threads: Option<NonZeroUsize>,

    #[arg(from_global)]
    verbose: u8,
}

fn input_files<P: AsRef<Path>>(user_input: &[P]) -> Result<Vec<PathBuf>> {
    Ok(user_input
        .iter()
        .flat_map(|user_path| {
            let user_path = user_path.as_ref();
            if user_path.is_dir() {
                match user_path.read_dir() {
                    Ok(iter) => iter
                        .map(|rd| rd.map(|entry| entry.path()))
                        .collect::<Vec<_>>(),
                    Err(e) => vec![Err(e)],
                }
            } else if user_path.is_file() {
                vec![Ok(user_path.to_owned())]
            } else {
                vec![]
            }
        })
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn serve(options: Options) -> Result<()> {
    info!(
        "Serve archive {:?} at {:?}",
        options.infiles, options.address,
    );
    let input_files = input_files(&options.infiles)?;
    let router = if input_files.len() == 1 {
        let inputfile = &input_files[0];
        let waj_server =
            waj::WajServer::open(inputfile).with_context(|| format!("Opening {:?}", inputfile))?;
        Box::new(waj_server) as Box<dyn waj::Router>
    } else {
        Box::new(waj::HostRouter::new(
            input_files
                .iter()
                .map(|f| -> anyhow::Result<_> {
                    let waj_server =
                        waj::WajServer::open(f).with_context(|| format!("Opening {:?}", f))?;
                    Ok((
                        f.file_name().unwrap().to_string_lossy().to_string(),
                        waj_server,
                    ))
                })
                .collect::<Result<HashMap<_, _>, _>>()?,
        ))
    };
    let server = waj::Server::new(router);

    Ok(server.serve(&options.address, options.threads)?)
}
