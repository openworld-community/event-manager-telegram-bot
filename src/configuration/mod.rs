mod raw_config;
pub mod config;

use std::{fs::File, io::prelude::*};
use clap::ArgMatches;
use config::Config;
use raw_config::RawConfiguration;

fn get_args<'a>() -> ArgMatches<'a> {
    clap::App::new("event-manager-telegram-bot")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("event-manager-telegram-bot")
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Configuration file")
                .takes_value(true)
                .default_value(""),
        )
        .get_matches()
}

fn parse_config_file(path: &str) -> RawConfiguration {
    let mut f = File::open(path).unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    toml::from_str::<RawConfiguration>(&contents)
        .map_err(|e| format!("Error loading configuration: {}", e))
        .unwrap()
}

pub fn get_config() -> Config {
    let args = get_args();
    let path = args.value_of("config").unwrap();
    Config::from(parse_config_file(path))
}

