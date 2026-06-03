mod context;
mod env;

use context::context::Context;
use env::env::Env;
use context::log::init_log;
use std::collections::HashMap;
use env::env::EnvType;
use ::log::LevelFilter;
use ::log::{debug, error, info, trace, warn};

fn main() {
    init_log(LevelFilter::Debug);
    debug!("OKOKOK");
    let user_env = HashMap::from([
        ("EPICS_CA_ADDR_LIST".to_string(), EnvType::StringArray(vec!["1.2.3.4".to_string()])),
        ("EPICS_CA_AUTO_ADDR_LIST".to_string(), EnvType::Boolean(true)),
    ]);
    let mut env = Env::new(user_env);
    let mut context = Context {
        env: env,
    };

    println!("{}", context.env);
}
