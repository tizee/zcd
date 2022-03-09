mod cli;
mod config;
mod db;
mod server;

use clap::Parser;
use cli::{AppExt, Cli};
use config::{config_exists, generate_config_file};

use std::process;

fn main() {
    if !config_exists() {
        generate_config_file();
    }
    let app = Cli::parse();
    if let Err(e) = app.run() {
        println!("{:?}", e);
        process::exit(1);
    }
}
