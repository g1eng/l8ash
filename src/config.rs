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
    integrity: Vec<String>,
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
            integrity: Vec::with_capacity(DEFAULT_MAX_COMMAND_LINE_CAPACITY),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    whitelist: Vec<AclTarget>,
    // blacklist: Vec<AclTarget>,
    // shell_integrity: String,
}

impl Drop for Config {
    fn drop(&mut self) {
        self.whitelist.clear();
        // self.blacklist.clear();
        // self.shell_integrity.clear();
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            whitelist: Vec::with_capacity(DEFAULT_WHITELIST_MAX_CAPACITY),
            // blacklist: Vec::with_capacity(DEFAULT_BLACKLIST_MAX_CAPACITY),
            // shell_integrity: String::with_capacity(DEFAULT_MAX_INTEGRITY_CAPACITY),
        }
    }

    /// blank config or not
    pub fn is_blank(&self) -> bool {
        self.whitelist.len() == 0
        // self.whitelist.len() == 0 && self.shell_integrity.is_empty()
    }

    /// Get command name if the command name is listed in the whitelist
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

    /// Get environmental variables for the specified pipeline alias
    pub fn get_env_vars(&self, pipeline_name: &str) -> io::Result<Vec<(String, String)>> {
        let mut res = Vec::with_capacity(DEFAULT_ENV_MAX_CAPACITY);
        for i in 0..self.whitelist.len() {
            if &self.whitelist[i].name == pipeline_name {
                // eprintln!("command name: {}", pipeline_name);
                for j in 0..self.whitelist[i].env.len() {
                    let t = self.whitelist[i].env[j].split_once('=').unwrap();
                    res.push((t.0.to_string(), t.1.to_string()));
                }
                return Ok(res);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no such pipeline: {}", pipeline_name),
        ))
    }

    /// Get integrity strings for the specified pipeline alias.
    /// The length of the returned vector is ensured to be equal to pipeline depth.
    pub fn get_integrities(&self, pipeline_name: &str) -> io::Result<Vec<&String>> {
        let mut res = Vec::with_capacity(DEFAULT_MAX_INTEGRITY_CAPACITY);
        for i in 0..self.whitelist.len() {
            if &self.whitelist[i].name == pipeline_name {
                for j in 0..self.whitelist[i].integrity.len() {
                    res.push(&self.whitelist[i].integrity[j]);
                }
                let con = self.whitelist[i]
                    .command_line
                    .split('|')
                    .collect::<Vec<&str>>()
                    .len();
                if res.len() == con {
                    return Ok(res);
                } else if res.len() == 0 {
                    return Ok(res);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("pipeline depth does not equal to the length of the integrity column. {}:{}", con, res.len()),
                    ));
                }
            }
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("no such pipeline: {}", pipeline_name),
        ))
    }
}

/// check whether the runtime configuration is exist or not
pub fn is_exist() -> bool {
    match File::open(PathBuf::from(format!(
        "{}/.leashrc",
        env::var("HOME").unwrap()
    ))) {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// generate Config instance from a local runtime configuration file
pub fn load() -> io::Result<Config> {
    let rcfile_path = match env::var("LEASH_CONF") {
        Ok(p) => p,
        Err(_) => format!("{}/.leashrc", env::var("HOME").unwrap()),
    };
    let f = File::open(PathBuf::from(rcfile_path))?;

    let s = io::read_to_string(BufReader::new(f))?;
    Ok(toml::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;

    #[test]

    fn test_new_blank() {
        let c = Config::new();
        assert!(c.is_blank());
    }

    #[test]
    fn test_load() {
        load().unwrap();
    }
    #[test]
    fn test_load_custom_config() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        load().unwrap();
    }

    #[test]
    fn test_get_white_command() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        assert_eq!(c.get_white_command("envg").unwrap(), "env | grep KORE");
    }

    #[test]
    fn test_get_white_command_error() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        assert!(c.get_white_command("mosomoso_nothing_there").is_err());
    }

    #[test]
    fn test_get_env_vars() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        let envvars = c.get_env_vars("envg").unwrap();
        assert_eq!(envvars.len(), 2);
    }

    #[test]
    fn test_get_no_env_vars() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        let envvars = c.get_env_vars("ls").unwrap();
        assert_eq!(envvars.len(), 0);
    }

    #[test]
    fn test_get_env_vars_for_invalid_command() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        assert!(c.get_env_vars("lsblk").is_err());
    }

    #[test]
    fn test_get_integrities() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        let envvars = c.get_integrities("envg").unwrap();
        assert_eq!(envvars.len(), 2);
    }

    #[test]
    fn test_get_integrities_for_invalid_command() {
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let c = load().unwrap();
        assert!(c.get_integrities("lsblk").is_err());
    }
}
