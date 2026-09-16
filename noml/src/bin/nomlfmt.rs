use std::env;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: nomlfmt <file.noml>");
        process::exit(1);
    };

    if let Some(flag) = args.next() {
        eprintln!("unexpected argument: {flag}");
        eprintln!("usage: nomlfmt <file.noml>");
        process::exit(1);
    }

    match noml::format::format_file_in_place(&path) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}
