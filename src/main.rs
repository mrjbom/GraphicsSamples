use log::LevelFilter;

fn main() {
    logger_init();
}

fn logger_init() {
    let mut builder = env_logger::builder();
    // Levels
    if cfg!(debug_assertions) {
        // Enable all message levels
        builder
            .filter_level(LevelFilter::Trace);
    } else {
        // Enable error, warn, info levels
        // debug and trace disabled
        builder
            .filter_level(LevelFilter::Info);
    }
    builder.format_timestamp(None);
    builder.format_target(false);
    builder.init();
}
