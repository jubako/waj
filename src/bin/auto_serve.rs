use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let address = match std::env::args_os().nth(1) {
        Some(a) => a,
        None => {
            eprintln!("No address specified. Please provide a address:port on which serve.");
            return ExitCode::FAILURE;
        }
    };

    let address = match address.to_str() {
        Some(a) => a,
        None => {
            eprintln!("Specified adresss is not valid utf-8.");
            return ExitCode::FAILURE;
        }
    };

    match env::current_exe() {
        Ok(exe_path) => {
            let server = waj::Server::new(exe_path);
            match server {
                Ok(server) => match server.serve(address) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        ExitCode::FAILURE
                    }
                },
                Err(waj::error::WajError::BaseError(_)) => {
                    eprintln!("Impossible to locate a Waj archive in the executable.");
                    eprintln!("This binary is not intented to be directly used, you must put a Waj archive at its end.");
                    ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("failed to get current exe path: {e}");
            ExitCode::FAILURE
        }
    }
}
