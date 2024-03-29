use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{ChildStdout, Command, Stdio};

use crate::config::{self, Config};
use crate::functions::calc_sha256sums;
use ring::test;
use ttyui::readline::Buffer;

use std::thread::sleep;
use std::time::Duration;

pub struct Shell {
    pipeline: Vec<Command>,
    env_kv: Vec<(String, String)>,
    raw_line: String,
    repr: String,
    acl: Config,
    pub debug: bool,
}

const MAX_PIPELINE_DEPTH: usize = 10;
const MAX_EXPRESSION_LENGTH: usize = 256;
const DEFAULT_ENV_MAX_CAPACITY: usize = 64;

const DEFAULT_PSSTRING: &str = "|x|> ";

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
            raw_line: String::new(),
            repr: String::new(),
            acl: Config::new(),
            debug: false,
        }
    }

    /// load runtime configuration for the shell instance
    ///
    pub fn load_conf(&mut self) -> io::Result<()> {
        self.acl = config::load()?;
        Ok(())
    }

    /// command and arguments parser
    ///
    fn parse_command(v: &mut Vec<&str>) -> Command {
        let mut cmd = Command::new(v[0]);
        v.remove(0);
        cmd.args(v);
        cmd
    }

    fn clear(&mut self) {
        self.pipeline.clear();
        self.raw_line.clear();
        self.repr.clear();
    }

    /// unified command executor
    ///
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
    ///
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
    ///
    fn parse_pipeline(&mut self) -> io::Result<()> {
        if self.pipeline.len() == 0 {
            return Ok(());
        }

        self.set_env_for_pipeline();
        if !self.acl.is_blank() {
            let integrity = self.acl.get_integrities(self.raw_line.trim())?;
            for i in 0..integrity.len() {
                let prog_name = self.pipeline[i].get_program().to_str().unwrap();
                let sum = calc_sha256sums(prog_name)?;
                let comp: Vec<u8> = test::from_hex(&integrity[i]).unwrap();
                if sum.as_ref() != comp {
                    eprintln!("invalid checksum for {}: {}", prog_name, &integrity[i]);
                    return Ok(());
                }
            }
        }
        let mut stdout_pipes: Vec<ChildStdout> = Vec::with_capacity(1);
        let current_command = match self.pipeline.len() {
            1 => &mut self.pipeline[0],
            _ => self.pipeline[0].stdout(Stdio::piped()),
        };

        let stdout = Self::exec(current_command);
        if self.pipeline.len() == 1 {
            return Ok(());
        }
        let stdout = stdout.unwrap();
        stdout_pipes.push(stdout);

        for i in 1..self.pipeline.len() {
            if i < self.pipeline.len() - 1 {
                let mut current_command = self.pipeline[i]
                    .stdout(Stdio::piped())
                    .stdin(Stdio::from(stdout_pipes.pop().unwrap()));
                if self.env_kv.len() != 0 {
                    for i in 0..self.env_kv.len() {
                        current_command = current_command.env(&self.env_kv[i].0, &self.env_kv[i].1);
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
        Ok(())
    }

    /// core routine to implement shared behavior for the commandline parser
    ///
    fn parse_commandline_core(&mut self, line: &str) -> io::Result<()> {
        self.raw_line = line.to_string();
        //blank line
        self.repr = self.raw_line.trim().to_string();
        if self.repr == "" {
            return Ok(());
        }
        //comment line
        if self.repr.starts_with("#") {
            self.raw_line.clear();
            self.repr.clear();
            return Ok(());
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
                    self.raw_line.clear();
                    self.repr.clear();
                    return Ok(());
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
        self.raw_line.clear();
        self.repr.clear();
        Ok(())
    }

    /// the parser facade for shell interpreter in batch mode
    ///
    pub fn parse_commandline_batch(&mut self, script_file: File) -> io::Result<()> {
        if self.debug {
            println!("config: {:?}", self.acl);
        }
        let mut b = BufReader::new(script_file);
        let mut s = String::with_capacity(MAX_EXPRESSION_LENGTH);
        loop {
            //EOF
            if b.read_line(&mut s)? == 0 {
                break;
            }
            self.parse_commandline_core(&s)?;
            s.clear();
        }
        Ok(())
    }

    /// the parser facade for shell interpreter in interactive mode
    ///
    pub fn parse_commandline_from_stdin(&mut self) -> io::Result<()> {
        if self.debug {
            println!("config: {:?}", self.acl);
        }
        println!();
        loop {
            let mut buf = Buffer::new();
            buf.set_prefix(DEFAULT_PSSTRING.to_string());
            buf.read_line()?;
            self.parse_commandline_core(buf.to_string().as_str())?;
            println!();
            // FIXME: the prefix string rendering is too fast to output and headed to
            // command output because the command stdout results asynchronous writing.
            sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod bdd {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    #[test]
    fn parser_core_can_parse_leash_compatible_shell_script() {
        let mut sh = Shell::new();
        let file = File::open(
            PathBuf::from("./fixtures/sample_posix_script.sh")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let mut s = String::new();
        let mut buf = BufReader::new(file);
        loop {
            if buf.read_line(&mut s).unwrap() == 0 {
                sh.parse_commandline_core(&s).unwrap();
                break;
            }
            sh.parse_commandline_core(&s).unwrap();
            s.clear();
        }
    }

    #[test]
    fn parser_core_can_parse_leash_script_with_restricted_mode() {
        let mut sh = Shell::new();
        env::set_var("LEASH_CONF", "./fixtures/example_leashrc");
        let file = File::open(
            PathBuf::from("./fixtures/sample_script.sh")
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let mut s = String::new();
        let mut buf = BufReader::new(file);
        sh.parse_commandline_core(&s).unwrap();
        loop {
            if buf.read_line(&mut s).unwrap() == 0 {
                sh.parse_commandline_core(&s).unwrap();
                break;
            }
            sh.parse_commandline_core(&s).unwrap();
            s.clear();
        }
    }
}
