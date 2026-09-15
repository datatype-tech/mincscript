use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (filename, source) = match args.next() {
        Some(path) => match fs::read_to_string(&path) {
            Ok(source) => (path, source),
            Err(error) => {
                eprintln!("failed to read {path}: {error}");
                return ExitCode::from(1);
            }
        },
        None => {
            let mut source = String::new();
            if let Err(error) = io::stdin().read_to_string(&mut source) {
                eprintln!("failed to read stdin: {error}");
                return ExitCode::from(1);
            }
            ("<stdin>".to_string(), source)
        }
    };

    match mincscript::parse(&source) {
        Ok(program) => {
            print!("{program}");
            ExitCode::SUCCESS
        }
        Err(diagnostics) => {
            mincscript::eprint_diagnostics(&filename, &source, &diagnostics);
            ExitCode::from(1)
        }
    }
}
