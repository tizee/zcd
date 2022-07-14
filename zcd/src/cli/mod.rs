mod client;

use anyhow::{Context, Result};
use client::Client;
use std::path::{Path, PathBuf};

use crate::config::{config_file, generate_config_file};
use crate::server::{check_server_alive, DbServer};

use clap::{ArgEnum, Args, Parser, Subcommand};
/// Zcd runs in two modes: cli and server.
/// By default it would use cli mode if server isn't running.
///
/// In cli mode, it behaves like zoxide.
///
/// In server mode, all instructions are sent to the db server running in the background.
/// You can use `zcd server <insruction>` to manage the server.

#[derive(Debug, Parser)]
#[clap(name="zcd",author, about="zcd server cli",long_about = None)]
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
    /// Server management
    #[clap(arg_required_else_help = true)]
    Server(ServerArgs),
    /// config management
    #[clap(arg_required_else_help = true)]
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs{
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
pub struct ServerArgs {
    #[clap(subcommand)]
    command: ServerCmds,
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

#[derive(Debug, Subcommand)]
enum ServerCmds {
    /// run server
    Run(DbConfigArgs),
    /// stop server
    Stop,
    /// restart server
    Restart(DbConfigArgs),
    /// check server status
    Status,
}

pub trait AppExt {
    fn run(&self) -> Result<()>;
}

impl AppExt for Cli {
    fn run(&self) -> Result<()> {
        match &self.command {
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
                        }else{
                            println!("{}", dir);
                        }
                    }
                }
            }
            Commands::Server(server) => {
                let server_cmd = &server.command;
                match server_cmd {
                    ServerCmds::Run(run_args) => {
                        let config_path = if run_args.path.is_some() {
                            PathBuf::from(run_args.path.as_ref().unwrap())
                        } else {
                            config_file().unwrap()
                        };
                        let server = DbServer::new(true, Path::new(config_path.as_path()))
                            .context("failed to init db server")?;
                        if run_args.daemon {
                            server.start_daemonized().context("failed to daemonize")?;
                        } else {
                            server.run().context("failed to run")?;
                        }
                    }
                    ServerCmds::Stop => {
                        if check_server_alive() {
                            println!("Stop server");
                        } else {
                            println!("Server isn't running");
                        }
                    }
                    ServerCmds::Restart(config_path) => {
                        if config_path.path.is_some() {
                            println!("Run server with {}", config_path.path.as_ref().unwrap());
                        } else {
                            println!("Run server");
                        }
                    }
                    ServerCmds::Status => {
                        if check_server_alive() {
                            // TODO
                            println!("Server is running for...");
                        } else {
                            println!("Server isn't running");
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
