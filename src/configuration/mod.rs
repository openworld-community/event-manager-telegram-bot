pub mod config;
mod raw_config;

use clap::ArgMatches;
use config::Config;
use raw_config::RawConfiguration;
use std::{fs::File, io::prelude::*};
use std::env::current_dir;
use std::path::Path;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    config: Option<String>,
}

fn get_args() -> Args {
    Args::parse()
}

fn parse_config_file<P: AsRef<Path>>(path: P) -> RawConfiguration {
    let mut f = File::open(path).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    toml::from_str::<RawConfiguration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e))
        .unwrap()
}

pub fn get_config() -> Config {
    let args = get_args();

    match args.config {
        Some(path) => {
            Config::from(parse_config_file(path))
        }
        None => {
            let mut path = current_dir().unwrap();
            path.push("config.toml");
            Config::from(parse_config_file(path))
        }
    }
}
