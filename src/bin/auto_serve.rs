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
            let waj_server = match waj::WajServer::open(&exe_path) {
                Ok(wj) => Box::new(wj),
                Err(waj::error::WajError::BaseError(_)) => {
                    eprintln!("Impossible to locate a Waj archive in the executable.");
                    eprintln!("This binary is not intented to be directly used, you must put a Waj archive at its end.");
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let server = waj::Server::new(waj_server);
            match server.serve(address, None) {
                Ok(()) => ExitCode::SUCCESS,
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
