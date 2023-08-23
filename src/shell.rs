use std::io;
use std::io::{stdin, BufRead, BufReader};
use std::process::{ChildStdout, Command, Stdio};

use crate::config::{self, Config};

pub struct Shell {
    pipeline: Vec<Command>,
    env_kv: Vec<(String, String)>,
    repr: String,
    acl: Config,
    pub debug: bool,
}

const MAX_PIPELINE_DEPTH: usize = 10;
const MAX_REPRESENTATION_LENGTH: usize = 256;
const DEFAULT_ENV_MAX_CAPACITY: usize = 64;

impl Drop for Shell {
    fn drop(&mut self) {
        self.clear();
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            pipeline: Vec::with_capacity(MAX_PIPELINE_DEPTH),
            env_kv: Vec::with_capacity(DEFAULT_ENV_MAX_CAPACITY),
            repr: String::with_capacity(MAX_REPRESENTATION_LENGTH),
            acl: Config::new(),
            debug: false,
        }
    }

    pub fn load_conf(&mut self) -> io::Result<()> {
        self.acl = config::load()?;
        Ok(())
    }

    /// command and arguments parser
    fn parse_command(v: &mut Vec<&str>) -> Command {
        let mut cmd = Command::new(v[0]);
        v.remove(0);
        cmd.args(v);
        cmd
    }

    fn clear(&mut self) {
        self.pipeline.clear();
        self.repr.clear();
    }

    /// unified command executor
    fn exec(c: &mut Command) -> io::Result<ChildStdout> {
        if let Some(r) = c.spawn()?.stdout {
            Ok(r)
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                format!("pipefail ({})", c.get_program().to_str().unwrap()),
            ))
        }
    }
    fn set_env_for_pipeline(&mut self) {
        for i in 0..self.pipeline.len() {
            if self.env_kv.len() != 0 {
                for j in 0..self.env_kv.len() {
                    self.pipeline[i].env(&self.env_kv[j].0, &self.env_kv[j].1);
                }
            }
        }
    }

    /// expression parser to execute command online
    fn parse_expression(&mut self) -> io::Result<()> {
        match self.pipeline.len() {
            0 => {}
            1 => {
                // for v in &self.env_kv {
                //     if self.debug {
                //         eprintln!("{}: {}", v.0, v.1)
                //     }
                //     self.pipeline[i].env_clear().env(&v.0, &v.1);
                // }
                if self.debug {
                    eprintln!("env_kv: {:?}", self.env_kv);
                }
                self.set_env_for_pipeline();
                if let Err(e) = self.pipeline[0].spawn() {
                    eprintln!("{}", e.to_string());
                }
            }
            _ => {
                self.set_env_for_pipeline();
                let mut stdout_pipes: Vec<ChildStdout> = Vec::with_capacity(1);
                let current_command = self.pipeline[0].stdout(Stdio::piped());
                if self.debug {
                    eprintln!("env_kv: {:?}", self.env_kv);
                }
                if self.env_kv.len() != 0 {
                    for i in 0..self.env_kv.len() {
                        current_command.env(&self.env_kv[i].0, &self.env_kv[i].1);
                    }
                }
                let out = Self::exec(current_command)?;
                stdout_pipes.push(out);
                for i in 1..self.pipeline.len() {
                    if i < self.pipeline.len() - 1 {
                        let mut current_command = self.pipeline[i]
                            .stdout(Stdio::piped())
                            .stdin(Stdio::from(stdout_pipes.pop().unwrap()));
                        if self.env_kv.len() != 0 {
                            for i in 0..self.env_kv.len() {
                                current_command =
                                    current_command.env(&self.env_kv[i].0, &self.env_kv[i].1);
                            }
                        }
                        let out = match Self::exec(current_command) {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("{}", e.to_string());
                                break;
                            }
                        };
                        stdout_pipes.push(out);
                    } else {
                        match self.pipeline[i]
                            .stdin(Stdio::from(stdout_pipes.pop().unwrap()))
                            .spawn()
                        {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("{}", e.to_string());
                                break;
                            }
                        };
                    }
                }
            }
        };
        Ok(())
    }

    /// the facade of parser for shell interpreter in interactive mode
    pub fn parse_pipeline_from_stdin(&mut self) -> io::Result<()> {
        if self.debug {
            println!("config: {:?}", self.acl);
        }
        let mut b = BufReader::new(stdin().lock());

        loop {
            //EOF
            if b.read_line(&mut self.repr)? == 0 {
                break;
            }
            //blank line
            self.repr = self.repr.trim().to_string();
            if self.repr == "" {
                continue;
            }
            //ACL (pipeline aliases)
            if self.acl.is_blank() == false {
                match self.acl.get_white_command(&self.repr.trim()) {
                    Ok(c) => {
                        self.env_kv = self.acl.get_env_vars(self.repr.trim()).unwrap();
                        self.repr = c;
                    }
                    Err(e) => {
                        eprintln!("{}", e.to_string());
                        self.repr.clear();
                        continue;
                    }
                }
            }
            if self.debug {
                println!("repr: {:?}", self.repr);
                println!("acl: {:?}", self.acl);
            }
            for p in self.repr.split('|') {
                let mut cmds = p
                    .split_whitespace()
                    .map(|s| s.trim())
                    .collect::<Vec<&str>>();
                self.pipeline.push(Self::parse_command(&mut cmds));
            }

            self.parse_expression()?;

            self.pipeline.clear();
            self.env_kv.clear();
            self.repr.clear();
        }
        Ok(())
    }
}
