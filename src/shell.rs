use std::fs::File;
use std::io;
use std::io::{stdin, BufRead, BufReader, Read};
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

    /// load runtime configuration for the shell instance
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

    /// set environmental variables for all command of the specified pipeline
    fn set_env_for_pipeline(&mut self) {
        if self.debug {
            eprintln!("env_kv: {:?}", self.env_kv);
        }
        for i in 0..self.pipeline.len() {
            if self.env_kv.len() != 0 {
                for j in 0..self.env_kv.len() {
                    self.pipeline[i].env(&self.env_kv[j].0, &self.env_kv[j].1);
                }
            }
        }
    }

    /// expression parser to execute command online
    fn parse_pipeline(&mut self) -> io::Result<()> {
        match self.pipeline.len() {
            0 => {}
            1 => {
                self.set_env_for_pipeline();
                if let Err(e) = self.pipeline[0].spawn() {
                    eprintln!("{}", e.to_string());
                }
            }
            _ => {
                self.set_env_for_pipeline();
                let mut stdout_pipes: Vec<ChildStdout> = Vec::with_capacity(1);
                let current_command = self.pipeline[0].stdout(Stdio::piped());
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

    /// core method to implement shared behavior for the commandline parser
    fn parse_commandline_core<R: Read>(&mut self, mut b: BufReader<R>) -> io::Result<()> {
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
            //comment line
            if self.repr.starts_with("#") {
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

            self.parse_pipeline()?;

            self.pipeline.clear();
            self.env_kv.clear();
            self.repr.clear();
        }
        Ok(())
    }

    /// the parser facade for shell interpreter in batch mode
    pub fn parse_commandline_batch(&mut self, script_file: File) -> io::Result<()> {
        if self.debug {
            println!("config: {:?}", self.acl);
        }
        let b = BufReader::new(script_file);
        self.parse_commandline_core(b)?;
        Ok(())
    }

    /// the parser facade for shell interpreter in interactive mode
    pub fn parse_commandline_from_stdin(&mut self) -> io::Result<()> {
        if self.debug {
            println!("config: {:?}", self.acl);
        }
        let b = BufReader::new(stdin().lock());
        self.parse_commandline_core(b)?;
        Ok(())
    }
}

