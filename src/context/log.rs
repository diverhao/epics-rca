use env_logger::Builder;
use ::log::LevelFilter;  // ::log means the external crate, not Context::log


pub fn init_log(level: LevelFilter) {
    Builder::new()
        .filter_level(LevelFilter::Debug)  // set level in code
        .init();
}


