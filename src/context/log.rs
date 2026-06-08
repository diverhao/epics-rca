use ::log::LevelFilter;
use env_logger::Builder; // ::log means the external crate, not Context::log

pub fn init_log(level: LevelFilter) {
    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .init();

    ::log::set_max_level(level);
}
