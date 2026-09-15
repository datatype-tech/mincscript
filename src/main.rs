use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(mincscript::cli::run(env::args().collect()) as u8)
}
