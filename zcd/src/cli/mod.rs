mod client;

use anyhow::{bail, Context, Result};
use client::Client;

use crate::config::generate_config_file;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// zcd – a simple jump navigation CLI tool.
#[derive(Debug, Parser)]
#[clap(name="zcd", author, about="zcd CLI tool", long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
    #[clap(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// insert or update an entry
    #[clap(arg_required_else_help = true)]
    Insert { entry: String },
    /// delete an entry
    #[clap(arg_required_else_help = true)]
    Delete { entry: String },
    /// query an entry based on keyword
    #[clap(arg_required_else_help = true)]
    Query(QueryArgs),
    /// list all entries
    List(ListArgs),
    /// merge entries from a z-compatible datafile
    #[clap(arg_required_else_help = true)]
    Import { path: PathBuf },
    /// write all entries to a z-compatible datafile
    #[clap(arg_required_else_help = true)]
    Export { path: PathBuf },
    /// config management
    #[clap(arg_required_else_help = true)]
    Config(ConfigArgs),
    /// clear all history
    Clear,
    /// display version information
    Version,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// show rank
    #[clap(short, long)]
    rank: bool,
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    entry: String,
    /// show rank
    #[clap(short, long)]
    rank: bool,
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true)]
pub struct ConfigArgs {
    /// generate default config file
    #[clap(long, short)]
    generate: bool,
}

pub trait AppExt {
    fn run(&self) -> Result<()>;
}

impl AppExt for Cli {
    fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Clear => {
                let mut client = Client::new().context("failed to create client")?;
                client.clear()?;
                println!("All entries have been cleared.");
            }
            Commands::Insert { entry } => {
                let mut client = Client::new().context("failed to create client")?;
                client.insert(entry)?;
            }
            Commands::Delete { entry } => {
                let mut client = Client::new().context("failed to create client")?;
                client.delete(entry)?;
            }
            Commands::Query(args) => {
                let client = Client::new().context("failed to create client")?;
                match client.query(&args.entry) {
                    Some(dir) => {
                        if args.rank {
                            println!("{:.2} {}", dir.rank, dir);
                        } else {
                            println!("{}", dir);
                        }
                    }
                    // Keep stdout clean: the shell plugin consumes stdout
                    // as the jump target.
                    None => bail!("no match found for {}", args.entry),
                }
            }
            Commands::Import { path } => {
                let mut client = Client::new().context("failed to create client")?;
                let count = client.import(path)?;
                println!("imported {} entries from {}", count, path.display());
            }
            Commands::Export { path } => {
                let client = Client::new().context("failed to create client")?;
                let count = client.export(path)?;
                println!("exported {} entries to {}", count, path.display());
            }
            Commands::List(list_args) => {
                let client = Client::new().context("failed to create client")?;
                for dir in client.list() {
                    if list_args.rank {
                        println!("{:.2} {}", dir.rank, dir);
                    } else {
                        println!("{}", dir);
                    }
                }
            }
            Commands::Config(config) => {
                if config.generate {
                    generate_config_file()?;
                }
            }
            Commands::Version => {
                println!("zcd version {}", env!("CARGO_PKG_VERSION"));
            }
        }
        Ok(())
    }
}
