use std::io;
use std::io::{stdin, BufRead, BufReader};
use std::process::{ChildStdout, Command, Stdio};

pub struct Shell {
    pipeline: Vec<Command>,
    repr: String,
}

const MAX_PIPELINE_DEPTH: usize = 10;
const MAX_REPRESENTATION_LENGTH: usize = 256;

impl Drop for Shell {
    fn drop(&mut self) {
        self.clear();
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            pipeline: Vec::with_capacity(MAX_PIPELINE_DEPTH),
            repr: String::with_capacity(MAX_REPRESENTATION_LENGTH),
        }
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

    /// expression parser to execute command online
    fn parse_expression(&mut self) -> io::Result<()> {
        match self.pipeline.len() {
            0 => {}
            1 => {
                if let Err(e) = self.pipeline[0].spawn() {
                    eprintln!("{}", e.to_string());
                }
            }
            _ => {
                let mut stdout_pipes: Vec<ChildStdout> = Vec::with_capacity(1);
                let out = Self::exec(self.pipeline[0].stdout(Stdio::piped()))?;
                stdout_pipes.push(out);
                for i in 1..self.pipeline.len() {
                    if i < self.pipeline.len() - 1 {
                        let out = match Self::exec(
                            self.pipeline[i]
                                .stdout(Stdio::piped())
                                .stdin(Stdio::from(stdout_pipes.pop().unwrap())),
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("{}", e.to_string());
                                break;
                            }
                        };
                        stdout_pipes.push(out);
                    } else {
                        match Self::exec(
                            self.pipeline[i].stdin(Stdio::from(stdout_pipes.pop().unwrap())),
                        ) {
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

    /// the facade of parser for shell interpreter
    pub fn parse_pipeline(&mut self) -> io::Result<()> {
        let mut b = BufReader::new(stdin().lock());

        loop {
            if b.read_line(&mut self.repr)? == 0 {
                break;
            }
            if self.repr.trim() == "" {
                continue;
            }
            for p in self.repr.split('|') {
                let mut cmds = p.split_whitespace().collect::<Vec<&str>>();
                self.pipeline.push(Self::parse_command(&mut cmds));
            }

            self.parse_expression()?;

            self.pipeline.clear();
            self.repr.clear();
        }
        Ok(())
    }
}
