use super::log::init_log;
use crate::env::env::Env;
use ::log::LevelFilter;
use ::log::info;

pub struct Context {
    pub env: Env,
    pub log_level: LevelFilter,
}

impl Context {
    pub fn new(user_env: Vec<(&str, &str)>, log_level: LevelFilter) -> Self {
        let context = Context {
            env: Env::new(user_env),
            log_level: log_level,
        };
        init_log(LevelFilter::Info);

        info!(
            "This EPICS client runs with following settings: \n{}",
            context.env
        );
        context
    }
}
