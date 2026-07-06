mod cli;
mod config;
mod db;

use clap::Parser;
use cli::{AppExt, Cli};
use config::{config_exists, generate_config_file};

use std::process;

fn main() {
    if !config_exists() {
        if let Err(e) = generate_config_file() {
            eprintln!("{:?}", e);
            process::exit(1);
        }
    }
    let app = Cli::parse();
    if let Err(e) = app.run() {
        eprintln!("{:?}", e);
        process::exit(1);
    }
}
