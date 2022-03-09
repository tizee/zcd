mod client;

use anyhow::{Context, Result};
use client::Client;

use crate::config::generate_config_file;

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
    /// install shell script
    Init(ShellTypes),
    /// insert or update an entry
    #[clap(arg_required_else_help = true)]
    Insert { entry: String },
    /// delete an entry
    #[clap(arg_required_else_help = true)]
    Delete { entry: String },
    /// query an entry based on keyword
    #[clap(arg_required_else_help = true)]
    Query { pattern: String },
    /// list all entries
    #[clap(arg_required_else_help = false)]
    List,
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

#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
pub struct ServerArgs {
    #[clap(subcommand)]
    command: ServerOps,
}

#[derive(Debug, Clone, Args)]
struct DbConfigArgs {
    /// use a specified zcd config file
    #[clap(long, short)]
    path: Option<String>,
}

#[derive(Debug, Clone, Args)]
#[clap(args_conflicts_with_subcommands = true)]
pub struct ConfigArgs {
    /// generate default config file
    #[clap(long, short)]
    generate: bool,
}

#[derive(Debug, Clone, Subcommand)]
enum ServerOps {
    /// run server
    #[clap(arg_required_else_help = true)]
    Run(DbConfigArgs),
    /// stop server
    #[clap(arg_required_else_help = true)]
    Stop,
    /// restart server
    #[clap(arg_required_else_help = true)]
    Restart(DbConfigArgs),
    /// check server status
    #[clap(arg_required_else_help = true)]
    Status,
}

pub trait AppExt {
    fn run(&self) -> Result<()>;
}

impl AppExt for Cli {
    fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Init(shell) => {
                let shell_type = &shell.shell;
                match shell_type {
                    ShellEnum::Zsh => {
                        println!("install script for zsh");
                    }
                    ShellEnum::Bash => {
                        println!("install script for bash");
                    }
                }
            }
            Commands::Insert { entry } => {
                let mut client = Client::new().context("failed to create client")?;
                client.insert(entry);
                println!("insert {}", entry);
            }
            Commands::Delete { entry } => {
                let mut client = Client::new().context("failed to create client")?;
                client.delete(entry);
                println!("delete {}", entry);
            }
            Commands::Query { pattern } => {
                let client = Client::new().context("failed to create client")?;
                if let Some(dir) = client.query(pattern) {
                    println!("{}", dir);
                } else {
                    println!("entry not found for {}", pattern);
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
            Commands::List => {
                println!("list entires");
                let client = Client::new().context("failed to create client")?;
                let list = client.list().context("failed to get list")?;
                for dir in list.into_iter() {
                    println!("{}", dir);
                }
            }
            Commands::Server(server) => {
                let server_cmd = &server.command;
                match server_cmd {
                    ServerOps::Run(config_path) => {
                        if config_path.path.is_some() {
                            println!("Run server with {}", config_path.path.as_ref().unwrap());
                        } else {
                            println!("Run server");
                        }
                    }
                    ServerOps::Stop => {
                        println!("Stop server");
                    }
                    ServerOps::Restart(config_path) => {
                        if config_path.path.is_some() {
                            println!("Run server with {}", config_path.path.as_ref().unwrap());
                        } else {
                            println!("Run server");
                        }
                    }
                    ServerOps::Status => {
                        println!("Server Status");
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
