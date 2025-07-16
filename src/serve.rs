use anyhow::{Context, Result};
use clap::Parser;
use core::num::NonZeroUsize;
use log::info;
use std::path::PathBuf;

/// Serve the waj archive on the web.
#[derive(Parser)]
pub struct Options {
    /// Archive to serve
    #[arg(value_parser)]
    infile: PathBuf,

    /// On which address serve the archive.
    #[arg(value_parser, default_value = "localhost:1234")]
    address: String,

    /// Number of threads to use to answer request
    #[arg(value_parser)]
    nb_threads: Option<NonZeroUsize>,

    #[arg(from_global)]
    verbose: u8,
}

pub fn serve(options: Options) -> Result<()> {
    info!(
        "Serve archive {:?} at {:?}",
        options.infile, options.address,
    );
    let server = waj::Server::new(&options.infile)
        .with_context(|| format!("Opening {:?}", options.infile))?;

    Ok(server.serve(&options.address, options.nb_threads)?)
}
