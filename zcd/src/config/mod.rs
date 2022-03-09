use std::char::ParseCharError;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .and_then(|h| if h.is_empty() { None } else { Some(h) })
        .map(PathBuf::from)
}

pub struct ConfigFile {
    pub config: Config,
    pub config_path: String,
}

#[derive(Debug)]
pub struct Config {
    /// lifetime in millisecond
    pub max_age: u64,
    /// debug mode
    pub debug: bool,
    /// paths to exclude for z
    pub exclude_dirs: Vec<String>,
    /// datafile path
    pub datafile: String,
}

pub struct ConfigBuilder {
    max_age: u64,
    debug: bool,
    exclude_dirs: Vec<String>,
    datafile: String,
}

impl ConfigBuilder {
    fn new() -> Self {
        let mut datafile = config_dir().unwrap();
        datafile.push(".zcddata");
        ConfigBuilder {
            max_age: 30000, // 5 * 60 * 1000
            debug: false,
            exclude_dirs: vec![],
            datafile: datafile.display().to_string(),
        }
    }
    pub fn max_age(&mut self, max_age: u64) -> &mut Self {
        self.max_age = max_age;
        self
    }

    pub fn debug(&mut self, debug: bool) -> &mut Self {
        self.debug = debug;
        self
    }

    pub fn exclude_dirs(&mut self, dirs: Vec<String>) -> &mut Self {
        self.exclude_dirs = dirs;
        self
    }

    pub fn datafile(&mut self, path: String) -> &mut Self {
        self.datafile = path;
        self
    }

    pub fn build(&mut self) -> Config {
        Config {
            max_age: self.max_age,
            debug: self.debug,
            exclude_dirs: self.exclude_dirs.clone(),
            datafile: self.datafile.clone(),
        }
    }
}

pub fn config_dir() -> Option<PathBuf> {
    let dir = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home_dir().map(|d| d.join(".config")));
    dir.map(|d| d.join("zcd"))
}

pub fn config_exists() -> bool {
    config_file().unwrap().exists()
}

pub fn config_file() -> Option<PathBuf> {
    Some(
        env::var("ZCD_CONFIG_FILE")
            .ok()
            .map(PathBuf::from)
            .filter(|config_path| config_path.is_file())
            .unwrap_or_else(|| config_dir().unwrap().join("config")),
    )
}

pub fn generate_config_file() {
    let config_file = config_file().unwrap();
    if config_file.exists() {
        println!(
            "The zcd config file already exists at: {}",
            config_file.to_string_lossy()
        );
        print!("Overwrite? (y/N): ");
        match io::stdout().flush() {
            Ok(_) => {}
            Err(error) => println!("{:?}", error),
        };
        let mut answer = String::new();
        match io::stdin().read_line(&mut answer) {
            Ok(_) => {}
            Err(error) => println!("{:?}", error),
        };
        if !answer.trim().eq_ignore_ascii_case("Y") {
            return;
        }
    } else {
        let config_dir = config_file.parent();
        match config_dir {
            Some(path) => match fs::create_dir_all(path) {
                Ok(_) => {}
                Err(error) => println!("{:?}", error),
            },
            None => {}
        }
    }

    let default_config = r#"#This is zcd's configuration file. You could define your zcd config here instead of putting it in your shell config files like bashrc etc.
# Specify how long the entry persists in seconds.
max_age=5000
# Datafile
datafile=~/.zcddata
# Exclude dirs
# eg. exclude_dirs=~/tmp,
exclude_dirs=[]
"#;
    match fs::write(&config_file, default_config) {
        Ok(_) => {
            println!(
                "Succesfully write zcd config file to {}",
                config_file.to_string_lossy()
            )
        }
        Err(error) => println!("{:?}", error),
    };
}

pub fn load_default_config() -> Result<Config> {
    load_config_from_path(config_file().unwrap())
}

pub fn load_config_from_path<P: AsRef<Path>>(path: P) -> Result<Config> {
    let path = path.as_ref();
    if path.exists() && path.is_file() {
        return match File::open(&path) {
            Ok(file) => read_config(file),
            Err(err) => Err(anyhow!(format!("{}: {}", path.display(), err))),
        };
    }
    Err(anyhow!(format!(
        "{}: doesn't exist or is not a regular file",
        path.display(),
    )))
}

#[derive(PartialEq, Eq, Clone, Debug)]
enum ConfigKeyWord {
    MaxAge,
    ExcludeDirs,
    Datafile,
    Debug,
    InvalidKeyword,
}

impl FromStr for ConfigKeyWord {
    type Err = ParseCharError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = s;
        let keyword = if key == "max_age" {
            ConfigKeyWord::MaxAge
        } else if key == "exclude_dirs" {
            ConfigKeyWord::ExcludeDirs
        } else if key == "debug" {
            ConfigKeyWord::Debug
        } else if key == "datafile" {
            ConfigKeyWord::Datafile
        } else {
            ConfigKeyWord::InvalidKeyword
        };
        Ok(keyword)
    }
}

fn parse_config(args: Vec<String>) -> Result<Config> {
    let mut builder = ConfigBuilder::new();

    for (_, arg) in args.into_iter().enumerate() {
        (|| -> Result<()> {
            let (key, value) = arg
                .split_once('=')
                .with_context(|| format!("invalid config on line: {}", arg))?;
            let keyword = ConfigKeyWord::from_str(key).unwrap();
            let res = match keyword {
                ConfigKeyWord::InvalidKeyword => Err(anyhow!("use an invalid config option!")),
                ConfigKeyWord::Debug => {
                    if value == "true" {
                        builder.debug(true);
                    } else if value == "false" {
                        builder.debug(false);
                    }
                    Ok(())
                }
                ConfigKeyWord::MaxAge => {
                    let val = value
                        .parse::<u64>()
                        .with_context(|| format!("invalid value for max_age: {}", value))?;
                    builder.max_age(val);
                    Ok(())
                }
                ConfigKeyWord::Datafile => {
                    let path = Path::new(value);
                    if path.is_dir() {
                        return Err(anyhow!("invalid config value for datafile: {}", value));
                    }
                    builder.datafile(path.display().to_string());
                    Ok(())
                }
                ConfigKeyWord::ExcludeDirs => {
                    let dirs = value
                        .trim_matches(|p| p == '[' || p == ']')
                        .split_terminator(',')
                        .filter(|&x| {
                            if !x.is_empty() {
                                let path = Path::new(x);
                                if path.is_dir() || path.is_symlink() {
                                    return true;
                                }
                                return false;
                            }
                            false
                        });
                    let paths = dirs
                        .map(|dir| String::from_str(dir).unwrap())
                        .collect::<Vec<String>>();
                    builder.exclude_dirs(paths.to_vec());
                    Ok(())
                }
            };
            res
        })()
        .with_context(|| format!("invalid config option for {}", arg))?;
    }

    Ok(builder.build())
}
fn read_config<R: Read>(config: R) -> Result<Config> {
    let reader = BufReader::new(config);
    let mut args = vec![];
    reader.lines().for_each(|line| {
        let line = String::from(line.unwrap().trim());
        // omit lines start with # or empty lines
        if line.is_empty() || line.as_bytes()[0] == b'#' {
            return;
        }
        args.push(line);
    });
    parse_config(args)
}

#[cfg(test)]
mod test_config {
    use super::*;
    #[test]
    fn test_config_file() {
        assert!(config_file().is_some());
    }
    #[test]
    fn test_config_dir() {
        assert!(config_dir().is_some());
    }

    #[test]
    fn test_read_config() {
        let config = read_config(
            &b"
    debug=true
# Specify how long the entry persists in seconds.
max_age=5000
    # Datafile
    datafile=~/.zcddata
# Exclude dirs
# eg. exclude_dirs=[/tmp]
exclude_dirs=[/tmp,/usr]
"[..],
        )
        .unwrap();
        assert_eq!(config.max_age, 5000);
        assert_eq!(config.datafile, "~/.zcddata");
        assert!(config.debug);
        assert_eq!(config.exclude_dirs.len(), 2);
    }

    #[test]
    fn test_config() {
        let mut data_file = config_dir().unwrap();
        data_file.push("zcd.bin");
        let config = ConfigBuilder::new()
            .exclude_dirs(vec![Path::new("~/.config").display().to_string()])
            .max_age(2000)
            .debug(true)
            .datafile(data_file.display().to_string())
            .build();
        assert_eq!(config.max_age, 2000);
        assert_eq!(config.datafile, data_file.display().to_string());
        assert!(config.debug);
        assert_eq!(
            config.exclude_dirs,
            vec![Path::new("~/.config").display().to_string()]
        );
    }
}
