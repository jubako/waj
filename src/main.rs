mod create;
mod list;
mod serve;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

const VERSION: &str = const_format::formatcp!(
    "{}{}",
    clap::crate_version!(),
    git_version::git_version!(
        args = ["--dirty=*", "--tags", "--always"],
        fallback = "",
        prefix = " (git:",
        suffix = ")"
    )
);

#[derive(Parser)]
#[command(name = "waj", author, version, long_version=VERSION, about, long_about=None)]
struct Cli {
    /// Set verbose level. Can be specify several times to augment verbose level.
    #[arg(short, long, action=clap::ArgAction::Count, global=true)]
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
    Serve(serve::Options),

    #[command(arg_required_else_help = true)]
    List(list::Options),
}

/// Serve the waj archive on the web.
#[derive(Parser)]
struct Serve {
    /// Archive to serve
    #[arg(value_parser)]
    infile: PathBuf,

    /// On which address serve the archive.
    #[arg(value_parser, default_value = "localhost:1234")]
    address: String,

    #[arg(from_global)]
    verbose: u8,
}

fn configure_log(verbose: u8) {
    let env = env_logger::Env::default()
        .filter("WAJ_LOG")
        .write_style("WAJ_LOG_STYLE");
    env_logger::Builder::from_env(env)
        .filter_module(
            "waj",
            match verbose {
                0 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Info,
                2 => log::LevelFilter::Debug,
                _ => log::LevelFilter::Trace,
            },
        )
        .format_module_path(false)
        .format_timestamp(None)
        .init();
}

fn run() -> Result<()> {
    let args = Cli::parse();
    configure_log(args.verbose);

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
            Commands::Create(options) => create::create(options),
            Commands::Serve(options) => serve::serve(options),
            Commands::List(options) => list::list(options),
        },
    }
}

fn main() -> ExitCode {
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .homepage(env!("CARGO_PKG_HOMEPAGE")));

    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error : {e:#}");
            ExitCode::FAILURE
        }
    }
}
