mod create;
mod list;

use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(name = "waj", author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    #[arg(
        long,
        num_args= 0..=1,
        default_missing_value = "",
        help_heading = "Advanced",
        value_parser([
            "",
            "create",
            "list",
            "serve",
        ])
    )]
    generate_man_page: Option<String>,

    #[arg(long, help_heading = "Advanced")]
    generate_complete: Option<clap_complete::Shell>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Create(create::Options),

    #[command(arg_required_else_help = true)]
    Serve(Serve),

    #[command(arg_required_else_help = true)]
    List(list::Options),
}

#[derive(Parser)]
struct Serve {
    #[arg(value_parser)]
    infile: PathBuf,

    #[arg(value_parser)]
    address: String,
}

fn main() -> jbk::Result<()> {
    let args = Cli::parse();

    if let Some(what) = args.generate_man_page {
        let command = match what.as_str() {
            "" => Cli::command(),
            "create" => create::Options::command(),
            "list" => list::Options::command(),
            "serve" => Serve::command(),
            _ => return Ok(Cli::command().print_help()?),
        };
        let man = clap_mangen::Man::new(command);
        man.render(&mut std::io::stdout())?;
        return Ok(());
    }

    if let Some(what) = args.generate_complete {
        let mut command = Cli::command();
        let name = command.get_name().to_string();
        clap_complete::generate(what, &mut command, name, &mut std::io::stdout());
        return Ok(());
    }

    match args.command {
        None => Ok(Cli::command().print_help()?),
        Some(c) => match c {
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
        },
    }
}
