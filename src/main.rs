mod config;
mod shell;

use shell::Shell;
use std::env;
use std::process::exit;

fn main() {
    let mut sh = Shell::new();
    if config::is_exist() {
        sh.load_conf().unwrap();
    }
    if let Some(s) = env::args().nth(1) {
        if s == "-d" || s == "--debug" {
            println!("debugging");
            sh.debug = true;
        }
    }
    if let Err(e) = sh.parse_pipeline_from_stdin() {
        eprintln!("{}", e.to_string());
        exit(1);
    }
}
