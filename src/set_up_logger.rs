use tracing::metadata::LevelFilter;

pub fn set_up_logger() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();
}
