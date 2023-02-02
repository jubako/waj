use jubako as jbk;

use clap::{Args, Parser, Subcommand};
use jim::Creator;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "jim")]
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
    Create(Create),

    #[clap(arg_required_else_help = true)]
    Serve(Serve),
}

#[derive(Args)]
struct Create {
    // Input
    #[clap(value_parser)]
    infiles: Vec<PathBuf>,

    // Archive name to create
    #[clap(short, long, value_parser)]
    outfile: PathBuf,

    #[clap(short, long, value_parser)]
    main_entry: PathBuf,
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
        Commands::Create(cmd) => {
            if args.verbose > 0 {
                println!("Creating archive {:?}", cmd.outfile);
                println!("With files {:?}", cmd.infiles);
            }

            let creator = Creator::new(&cmd.outfile, cmd.main_entry);
            creator.run(cmd.outfile, cmd.infiles)
        }

        Commands::Serve(serve_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Serve archive {:?} at {:?}",
                    serve_cmd.infile, serve_cmd.address,
                );
            }
            let server = jim::Server::new(serve_cmd.infile)?;
            server.serve(&serve_cmd.address)
        }
    }
}
