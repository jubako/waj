use jubako as jbk;
use libwaj as waj;

mod create;
mod list;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "waj")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(arg_required_else_help = true)]
    Create(create::Options),

    #[clap(arg_required_else_help = true)]
    Serve(Serve),

    #[clap(arg_required_else_help = true)]
    List(list::Options),
}

#[derive(Args)]
struct Serve {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    address: String,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Create(options) => create::create(options, args.verbose),
        Commands::Serve(serve_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Serve archive {:?} at {:?}",
                    serve_cmd.infile, serve_cmd.address,
                );
            }
            let server = waj::Server::new(serve_cmd.infile)?;
            server.serve(&serve_cmd.address)
        }
        Commands::List(options) => list::list(options),
    }
}
