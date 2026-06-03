mod cli;
mod config;
mod datetime;
mod editor;
mod models;
mod output;
mod service;
mod storage;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
