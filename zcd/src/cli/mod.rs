mod client;

use anyhow::{Context, Result};
use client::Client;

use crate::config::{config_file, generate_config_file};

use clap::{ArgEnum, Args, Parser, Subcommand};

/// zcd – a simple jump navigation CLI tool.

#[derive(Debug, Parser)]
#[clap(name="zcd",author, about="zcd CLI tool",long_about = None)]
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
    #[clap(arg_required_else_help = false)]
    List(ListArgs),
    /// Import data from datafile
    #[clap(arg_required_else_help = true)]
    Import(ImportExportArgs),
    /// Export data into datafile
    #[clap(arg_required_else_help = true)]
    Export(ImportExportArgs),
    /// config management
    #[clap(arg_required_else_help = true)]
    Config(ConfigArgs),
    /// clear all history
    Clear,
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
pub struct ShellTypes {
    #[clap(long, arg_enum)]
    shell: ShellEnum,
}

#[derive(Debug, Clone, ArgEnum)]
enum ShellEnum {
    Zsh,
    Bash,
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true)]
pub struct ImportExportArgs {
    path: String,
    #[clap(short, long, arg_enum)]
    format: DataFileFormat,
}

#[derive(Debug, Clone, ArgEnum)]
enum DataFileFormat {
    Z,
    Zcd,
}

#[derive(Debug, Args)]
struct DbConfigArgs {
    /// use a specified zcd config file
    #[clap(long, short)]
    path: Option<String>,
    /// run as daemon
    #[clap(long, short)]
    daemon: bool,
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
                if let Ok(Some(dir)) = client.query(&args.entry) {
                    if args.rank {
                        println!("{:.2} {}", dir.rank, dir);
                    } else {
                        println!("{}", dir);
                    }
                } else {
                    println!("entry not found for {}", args.entry);
                }
            }
            Commands::Import(import_args) => {
                let import_format = &import_args.format;
                match import_format {
                    DataFileFormat::Z => {
                        println!("import z datafile {}", import_args.path);
                    }
                    DataFileFormat::Zcd => {
                        println!("import zcd datafile {}", import_args.path);
                    }
                }
            }
            Commands::Export(export_args) => {
                let export_format = &export_args.format;
                match export_format {
                    DataFileFormat::Z => {
                        println!("import z datafile {}", export_args.path);
                    }
                    DataFileFormat::Zcd => {
                        println!("import zcd datafile {}", export_args.path);
                    }
                }
            }
            Commands::List(list_args) => {
                let client = Client::new().context("failed to create client")?;
                let res = client.list().context("failed to get list")?;
                if let Some(list) = res {
                    for dir in list.into_iter() {
                        if list_args.rank {
                            println!("{:.2} {}", dir.rank, dir);
                        } else {
                            println!("{}", dir);
                        }
                    }
                }
            }
            Commands::Config(config) => {
                if config.generate {
                    generate_config_file();
                }
            }
        }
        Ok(())
    }
}
