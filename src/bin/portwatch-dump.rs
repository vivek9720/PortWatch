use std::env;
use std::fs;

fn main() {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: portwatch-dump <exchange-file>");
            std::process::exit(2);
        }
    };

    let data = match fs::read(&path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("failed to read {path}: {err}");
            std::process::exit(1);
        }
    };

    match portwatch::parse(&data) {
        Ok(report) => println!("{report:#?}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
