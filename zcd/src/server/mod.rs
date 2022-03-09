mod ops;
use std::fmt;
use std::fmt::Display;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::db::Database;

pub static SOCKET_PATH: &str = "/tmp/zcd-socket";
pub struct DbServer<'a> {
    inner: UnixListener,
    db: Database<'a>,
    debug: bool,
}

// client side
pub fn check_server_alive() -> bool {
    let socket_file = Path::new(SOCKET_PATH);
    UnixStream::connect(socket_file).is_ok()
}

fn handle_socket(res: io::Result<UnixStream>) -> Option<UnixStream> {
    match res {
        Ok(socket) => Some(socket),
        Err(e) => {
            eprintln!("{}", e);
            None
        }
    }
}

pub(crate) enum ServerOps {
    Start,
    Restart,
    Stop,
    Status,
    Query,
    Delete,
    Insert,
    InvalidOp,
}

// client side
impl Display for ServerOps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerOps::Start => write!(f, "start"),
            ServerOps::Restart => write!(f, "restart"),
            ServerOps::Stop => write!(f, "stop"),
            ServerOps::Status => write!(f, "status"),
            ServerOps::Query => write!(f, "query"),
            ServerOps::Delete => write!(f, "delete"),
            ServerOps::Insert => write!(f, "insert"),
            ServerOps::InvalidOp => write!(f, ""),
        }
    }
}

// server side parsing
impl FromStr for ServerOps {
    type Err = std::char::ParseCharError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = s;
        let op = if key == "start" {
            ServerOps::Start
        } else if key == "restart" {
            ServerOps::Restart
        } else if key == "stop" {
            ServerOps::Stop
        } else if key == "status" {
            ServerOps::Status
        } else if key == "query" {
            ServerOps::Query
        } else if key == "delete" {
            ServerOps::Delete
        } else if key == "insert" {
            ServerOps::Insert
        } else {
            ServerOps::InvalidOp
        };
        Ok(op)
    }
}

fn handle_incoming(mut stream: UnixStream, debug: bool) {
    let reader = BufReader::new(stream.try_clone().unwrap());
    for message in reader.lines() {
        let message = message.unwrap();
        if debug {
            println!("{}", message);
        }
        // parse message
        let args: Vec<&str> = message.split(' ').filter(|x| !x.is_empty()).collect();
        if !args.is_empty() {
            let op = ServerOps::from_str(args[0]).unwrap();
            match op {
                ServerOps::Start => {
                    // do nothing as server has been already running
                }
                ServerOps::Restart => {
                    // restart server
                }
                ServerOps::Stop => {
                    // exit server process gracefully
                }
                ServerOps::Status => {}
                ServerOps::Query => {}
                ServerOps::Delete => {}
                ServerOps::Insert => {}
                ServerOps::InvalidOp => {}
            }
        }
    }
}

impl DbServer<'_> {
    fn new(debug: bool, config_path: &Path) -> Result<Self> {
        let socket_file = Path::new(SOCKET_PATH);
        if socket_file.exists() {
            fs::remove_file(socket_file).context("failed to open socket for server")?;
        }
        let listener =
            UnixListener::bind(socket_file).context("failed to bind socket for server")?;
        let db = Database::new(config_path).context("failed to init database for zcd server")?;
        Ok(DbServer {
            inner: listener,
            debug,
            db,
        })
    }

    fn run(&self) {
        for client in self.inner.incoming().filter_map(handle_socket) {
            let debug = self.debug;
            // use thread handle incoming message for each client connection
            thread::spawn(move || handle_incoming(client, debug));
        }
    }
}
