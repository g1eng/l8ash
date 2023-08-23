use serde_derive::Deserialize;
use std::env;
use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use toml;

const DEFAULT_WHITELIST_MAX_CAPACITY: usize = 128;
const DEFAULT_BLACKLIST_MAX_CAPACITY: usize = 256;

const DEFAULT_MAX_COMMAND_NAME_CAPACITY: usize = 32;
const DEFAULT_MAX_COMMAND_LINE_CAPACITY: usize = 128;
const DEFAULT_ENV_MAX_CAPACITY: usize = 64;
const DEFAULT_MAX_INTEGRITY_CAPACITY: usize = 128;

#[derive(Deserialize, Debug)]
struct AclTarget {
    name: String,
    command_line: String,
    pub env: Vec<String>,
    integrity: String,
}

impl Drop for AclTarget {
    fn drop(&mut self) {
        self.name.clear();
        self.command_line.clear();
        self.env.clear();
        self.integrity.clear();
    }
}

impl AclTarget {
    pub fn new() -> Self {
        AclTarget {
            name: String::with_capacity(DEFAULT_MAX_COMMAND_NAME_CAPACITY),
            command_line: String::with_capacity(DEFAULT_MAX_COMMAND_LINE_CAPACITY),
            env: Vec::with_capacity(DEFAULT_ENV_MAX_CAPACITY),
            integrity: String::with_capacity(DEFAULT_MAX_INTEGRITY_CAPACITY),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    whitelist: Vec<AclTarget>,
    // blacklist: Vec<AclTarget>,
    shell_integrity: String,
}

impl Drop for Config {
    fn drop(&mut self) {
        self.whitelist.clear();
        // self.blacklist.clear();
        self.shell_integrity.clear();
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            whitelist: Vec::with_capacity(DEFAULT_WHITELIST_MAX_CAPACITY),
            // blacklist: Vec::with_capacity(DEFAULT_BLACKLIST_MAX_CAPACITY),
            shell_integrity: String::with_capacity(DEFAULT_MAX_INTEGRITY_CAPACITY),
        }
    }

    pub fn is_blank(&self) -> bool {
        self.whitelist.len() == 0 && self.shell_integrity.is_empty()
    }

    pub fn get_white_command(&self, command: &str) -> io::Result<String> {
        for i in 0..self.whitelist.len() {
            if &self.whitelist[i].name == command {
                return Ok(self.whitelist[i].command_line.clone());
            }
        }
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied",
        ))
    }

    pub fn get_env_vars(&self, command_name: &str) -> io::Result<Vec<(String, String)>> {
        let mut res = Vec::with_capacity(DEFAULT_ENV_MAX_CAPACITY);
        for i in 0..self.whitelist.len() {
            eprintln!(
                "evaluating: {} == {}",
                &self.whitelist[i].name, command_name
            );
            if &self.whitelist[i].name == command_name {
                eprintln!("command name: {}", command_name);
                for j in 0..self.whitelist[i].env.len() {
                    let t = self.whitelist[i].env[j].split_once('=').unwrap();
                    res.push((t.0.to_string(), t.1.to_string()));
                }
                return Ok(res);
            }
        }
        eprintln!("no env");
        Ok(Vec::with_capacity(0))
    }
}

pub fn is_exist() -> bool {
    match File::open(PathBuf::from(format!(
        "{}/.leashrc",
        env::var("HOME").unwrap()
    ))) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn load() -> io::Result<Config> {
    let f = File::open(PathBuf::from(format!(
        "{}/.leashrc",
        env::var("HOME").unwrap()
    )))?;

    let s = io::read_to_string(BufReader::new(f))?;
    Ok(toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?)
}
