mod shell;
use std::process::exit;

fn main() {
    if let Err(e) = shell::Shell::new().parse_pipeline() {
        eprintln!("{}", e.to_string());
        exit(1);
    }
}
