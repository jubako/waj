use jubako as jbk;
use libwaj as waj;

use clap::Parser;
use std::env;
use std::process::ExitCode;

#[derive(Parser)]
#[clap(name = "waj")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(value_parser)]
    address: String,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    match env::current_exe() {
        Ok(exe_path) => {
            let server = waj::Server::new(exe_path);
            match server {
                Ok(server) => match server.serve(&args.address) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::FAILURE
                    }
                },
                Err(e) => match e.error {
                    jbk::ErrorKind::NotAJbk => {
                        eprintln!("Impossible to locate a Waj archive in the executable.");
                        eprintln!("This binary is not intented to be directly used, you must put a Waj archive at its end.");
                        ExitCode::FAILURE
                    }
                    _ => {
                        eprintln!("Error: {e}");
                        ExitCode::FAILURE
                    }
                },
            }
        }
        Err(e) => {
            eprintln!("failed to get current exe path: {e}");
            ExitCode::FAILURE
        }
    }
}
