use clap::CommandFactory;
use std::path::Path;

#[path = "src/cli.rs"]
mod cli;

fn main() {
    let out_dir = match std::env::var_os("OUT_DIR") {
        Some(out) => out,
        None => return,
    };
    let out_dir = Path::new(&out_dir);

    let cmd = cli::Cli::command();
    let name = cmd.get_name().to_string();

    // Generate man page for the main command
    let man = clap_mangen::Man::new(cmd);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer).unwrap();
    std::fs::write(out_dir.join(format!("{}.1", name)), buffer).unwrap();
}
