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
}

#[derive(Args)]
struct Serve {
    #[clap(value_parser)]
    infile: PathBuf,

    #[clap(value_parser)]
    address: String,

    #[clap(value_parser)]
    port: u16,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Create(create_cmd) => {
            if args.verbose > 0 {
                println!("Creating archive {:?}", create_cmd.outfile);
                println!("With files {:?}", create_cmd.infiles);
            }

            let creator = Creator::new(&create_cmd.outfile);
            creator.run(create_cmd.outfile, create_cmd.infiles)
        }

        Commands::Serve(serve_cmd) => {
            if args.verbose > 0 {
                println!(
                    "Serve archive {:?} at {:?}:{}",
                    serve_cmd.infile, serve_cmd.address, serve_cmd.port
                );
            }

            jim::serve(serve_cmd.infile, &serve_cmd.address, serve_cmd.port)
        }
    }
}
