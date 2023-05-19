use clap::ArgMatches;

pub fn get_args() -> ArgMatches {
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
