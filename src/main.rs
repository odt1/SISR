use sisr::logging;
use tracing::info;

fn main() {
    logging::setup();
    info!("Starting SISR...");

    let config = sisr::config::Config::parse();

    logging::set_level(config.log.level.as_ref().unwrap().parse().unwrap());

    if let Some(log_file) = &config.log.log_file
        && let Some(path) = &log_file.path
    {
        let level = log_file
            .file_level
            .as_ref()
            .unwrap_or(&config.log.level.as_ref().unwrap().parse().unwrap())
            .parse()
            .unwrap();
        logging::add_file(path, level);
    }
    info!("CLI arguments parsed: {:?}", config);
}
