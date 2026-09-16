use std::env;
use std::fs;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| {
        eprintln!("usage: noml <check|format|parse> [file]");
        process::exit(1);
    });

    match command.as_str() {
        "parse" => {
            let path = args.next().unwrap_or_else(|| {
                eprintln!("usage: noml parse <file.noml>");
                process::exit(1);
            });
            let text = fs::read_to_string(&path).unwrap_or_else(|error| {
                eprintln!("failed to read {path}: {error}");
                process::exit(1);
            });
            match noml::parse(&text) {
                Ok(value) => println!("{}", noml::serialize(&value)),
                Err(error) => {
                    eprintln!("parse failed: {error}");
                    process::exit(1);
                }
            }
        }
        "format" => {
            let path = args.next().unwrap_or_else(|| {
                eprintln!("usage: noml format <file.noml>");
                process::exit(1);
            });
            match noml::format::format_file_in_place(&path) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
        }
        "check" => {
            let path = args.next().unwrap_or_else(|| {
                eprintln!("usage: noml check <file.noml>");
                process::exit(1);
            });
            let text = fs::read_to_string(&path).unwrap_or_else(|error| {
                eprintln!("failed to read {path}: {error}");
                process::exit(1);
            });
            if noml::parse(&text).is_ok() {
                println!("{path}: OK");
            } else {
                eprintln!("{path}: invalid NOML");
                process::exit(1);
            }
        }
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: noml <check|format|parse> [file]");
            process::exit(1);
        }
    }
}
