mod config;
mod functions;
mod shell;

use shell::Shell;
use std::env;
use std::fs::File;

fn main() {
    let mut sh = Shell::new();
    if config::is_exist() {
        sh.load_conf().unwrap();
    }
    let mut target_arg_n: usize = 1;
    if let Some(s) = env::args().nth(target_arg_n) {
        if s == "-d" || s == "--debug" {
            println!("debugging");
            sh.debug = true;
            target_arg_n += 1;
        }
    }

    match env::args().nth(target_arg_n) {
        Some(f) => {
            let file = File::open(f.as_str()).expect(format!("file {} cannot open", f).as_str());
            sh.parse_commandline_batch(file)
                .expect("unconditional failure");
        }
        None => {
            sh.parse_commandline_from_stdin()
                .expect("unconditional failure");
        }
    };
}
