use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Options {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(from_global)]
    verbose: u8,
}

struct Lister;
use waj::CommonEntry;

impl waj::walk::Operator<(), waj::FullBuilder> for Lister {
    fn on_start(&self, _context: &mut ()) -> jbk::Result<()> {
        Ok(())
    }
    fn on_stop(&self, _context: &mut ()) -> jbk::Result<()> {
        Ok(())
    }
    fn on_content(&self, _context: &mut (), entry: &waj::Content) -> jbk::Result<()> {
        let path = String::from_utf8_lossy(entry.path());
        println!("{:?}", path);
        Ok(())
    }
    fn on_redirect(&self, _context: &mut (), entry: &waj::Redirect) -> jbk::Result<()> {
        let path = String::from_utf8_lossy(entry.path());
        println!("{:?}", path);
        Ok(())
    }
}

pub fn list(options: Options) -> Result<()> {
    let waj =
        waj::Waj::new(&options.infile).with_context(|| format!("Opening {:?}", options.infile))?;
    let mut walker = waj::walk::Walker::new(&waj, ());
    Ok(walker.run(&Lister)?)
}
