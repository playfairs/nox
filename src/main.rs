use nox::core::{error::Error, output};

fn main() {
    match nox::cli::run() {
        Ok(()) => {}
        Err(Error::Exit(code)) => std::process::exit(code),
        Err(error) => {
            output::error(format!("nox: {error}"));
            std::process::exit(1);
        }
    }
}
