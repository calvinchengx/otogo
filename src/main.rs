use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(otogo::cli::main() as u8)
}
