mod ops;

use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
// use std::thread;
use crossbeam_utils::thread;

use daemonize::Daemonize;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use anyhow::anyhow;
use anyhow::{Context, Result};

use crate::db::Dir;
use crate::db::{Database, OpsDelegate};

pub static SOCKET_PATH: &str = "/tmp/zcd-socket";
#[derive(Clone)]
pub struct DbServer<'a> {
    db: Arc<Mutex<Database<'a>>>,
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

#[derive(Serialize, Deserialize)]
pub enum ServerOps {
    Restart,
    Stop,
    Status,
    Query(String),
    Delete(String),
    Insert(String),
    List,
}

impl<'a> DbServer<'a> {
    pub fn new(debug: bool, config_path: &Path) -> Result<Self> {
        let db = Database::new(config_path).context("failed to init database for zcd server")?;
        Ok(DbServer {
            debug,
            db: Arc::new(Mutex::new(db)),
        })
    }
    fn listen(&self) -> Result<()> {
        let socket_file = Path::new(SOCKET_PATH);
        if socket_file.exists() {
            fs::remove_file(socket_file).context("failed to open socket for server")?;
        }
        let listener =
            UnixListener::bind(socket_file).context("failed to bind socket for server")?;
        let debug = self.debug;
        loop {
            let (stream, _) = listener.accept().context("failed to create connection")?;
            thread::scope(|s| {
                s.spawn(|_| self.handle_connection(stream, debug));
            })
            .unwrap();
        }
    }

    pub fn run(&self) -> Result<()> {
        self.listen()
    }
    pub fn start_daemonized(&self) -> Result<()> {
        let stdout = File::create("/tmp/zcdserver.out")?;
        let stderr = File::create("/tmp/zcdserver.err")?;
        let daemonize = Daemonize::new()
            .pid_file("/tmp/zcdserver.pid")
            .stdout(stdout)
            .stderr(stderr)
            .working_directory("/tmp");
        match daemonize.start() {
            Ok(_) => {
                self.listen();
                Ok(())
            }
            Err(e) => Err(anyhow!("{}", e)),
        }
    }

    fn handle_connection(&self, stream: UnixStream, debug: bool) -> Result<()> {
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        let mut buf = Vec::new();
        let msg = get_message(&mut reader, &mut buf)?;

        match msg {
            Some(ServerOps::Insert(path)) => {
                println!("insert {}", path);
                self.db.lock().unwrap().insert_or_update(path.into());
                Ok(())
            }
            Some(ServerOps::Delete(path)) => {
                println!("delete {}", path);
                self.db.lock().unwrap().delete(path.as_str());
                Ok(())
            }
            Some(ServerOps::Query(pattern)) => {
                println!("query {}", pattern);
                if let Some(dir) = self.db.lock().unwrap().query(pattern.as_str()) {
                    let mut writer = BufWriter::new(stream);
                    send_message(&mut writer, dir)?;
                }
                Ok(())
            }
            Some(ServerOps::List) => {
                println!("get list");
                if let Some(list) = self.db.lock().unwrap().list() {
                    let mut writer = BufWriter::new(stream);
                    send_message(&mut writer, list)?;
                }
                Ok(())
            }
            Some(ServerOps::Status) => Ok(()),
            Some(ServerOps::Stop) => Ok(()),
            Some(ServerOps::Restart) => Ok(()),
            None => {
                // invalid op, just no-op
                Ok(())
            }
        }
    }
}

pub struct DbClient {
    inner: UnixStream,
}

impl DbClient {
    pub fn new() -> Result<Self> {
        let socket_file = Path::new(SOCKET_PATH);
        // create socket
        let client_socket =
            UnixStream::connect(socket_file).context("failed to create connection for client")?;
        Ok(DbClient {
            inner: client_socket,
        })
    }
    pub fn insert(&self, path: &str) -> Result<()> {
        let mut writer = BufWriter::new(self.inner.try_clone().unwrap());
        send_message(&mut writer, ServerOps::Insert(path.to_string()))
            .context("failed to send inert message")?;
        Ok(())
    }
    pub fn delete(&self, path: &str) -> Result<()> {
        let mut writer = BufWriter::new(self.inner.try_clone().unwrap());
        send_message(&mut writer, ServerOps::Insert(path.to_string()))
            .context("failed to send inert message")?;
        Ok(())
    }
    pub fn query(&self, pattern: &str) -> Result<Option<Vec<Dir<'static>>>> {
        let mut reader = BufReader::new(self.inner.try_clone().unwrap());
        let mut writer = BufWriter::new(self.inner.try_clone().unwrap());
        send_message(&mut writer, ServerOps::Query(pattern.to_string()))
            .context("failed to send inert message")?;
        let mut buf = Vec::new();
        let msg = get_message(&mut reader, &mut buf)?;
        match msg {
            Some(list) => Ok(Some(list)),
            None => Ok(None),
        }
    }
    pub fn list(&self) -> Result<Option<Vec<Dir<'static>>>> {
        let mut reader = BufReader::new(self.inner.try_clone().unwrap());
        let mut writer = BufWriter::new(self.inner.try_clone().unwrap());
        send_message(&mut writer, ServerOps::List).context("failed to send inert message")?;
        let mut buf = Vec::new();
        let msg = get_message(&mut reader, &mut buf)?;
        match msg {
            Some(dir_list) => Ok(Some(dir_list)),
            None => Ok(None),
        }
    }
}
fn get_message<T: DeserializeOwned>(
    reader: &mut BufReader<UnixStream>,
    buf: &mut Vec<u8>,
) -> Result<Option<T>> {
    buf.clear();
    reader.read_until(0, buf).context("failed to get message")?;
    if buf.is_empty() {
        // no-op
        return Ok(None);
    }
    if buf.as_slice().last() == Some(&0) {
        buf.pop();
    }
    Ok(serde_json::from_slice(&buf).context("failed to parse")?)
}

fn send_message<T: Serialize>(stream: &mut BufWriter<UnixStream>, msg: T) -> Result<()> {
    let message = serde_json::to_vec(&msg).context("failed to serialize data into json format")?;
    stream
        .write_all(&message)
        .context("failed to send message")?;
    stream.write_all(&[0]).context("failed to send eof");
    // make sure send
    stream.flush().context("failed to flush")?;
    Ok(())
}
