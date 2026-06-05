use super::log::init_log;
use crate::env::env::Env;
use ::log::LevelFilter;
use ::log::info;
use std::sync::Mutex;
use std::sync::OnceLock;

// Declare the global — starts empty, set once
pub static CONTEXT: OnceLock<Mutex<Context>> = OnceLock::new();

pub struct Context {
    pub env: Env,
    pub log_level: LevelFilter,
}

impl Context {
    pub fn create(user_env: Vec<(&str, &str)>, log_level: LevelFilter) -> () {
        let context = Context {
            env: Env::new(user_env),
            log_level: log_level,
        };
        init_log(LevelFilter::Info);

        info!(
            "This EPICS client runs with following settings: \n{}",
            context.env
        );
        CONTEXT.set(Mutex::new(context)).ok();
        let ctx: std::sync::MutexGuard<'static, Context> = CONTEXT.get().unwrap().lock().unwrap();
        // ctx
    }
}

pub fn create_context(user_env: Vec<(&str, &str)>, log_level: LevelFilter) {
    Context::create(user_env, log_level);
}

pub fn get_context() -> std::sync::MutexGuard<'static, Context> {
    CONTEXT.get().unwrap().lock().unwrap()
}
