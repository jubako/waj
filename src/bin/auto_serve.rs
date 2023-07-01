use jubako as jbk;

use clap::Parser;
use std::env;
use std::process::ExitCode;

#[derive(Parser)]
#[clap(name = "wpack")]
#[clap(author, version, about, long_about=None)]
struct Cli {
    #[clap(value_parser)]
    address: String,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    match env::current_exe() {
        Ok(exe_path) => {
            let server = wpack::Server::new(exe_path);
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
                        eprintln!("Impossible to locate a Wpack archive in the executable.");
                        eprintln!("This binary is not intented to be directly used, you must put a Wpack archive at its end.");
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
