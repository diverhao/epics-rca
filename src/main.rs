mod context;
mod env;
mod udp;

use context::context::Context;
use env::env::Env;
use context::log::init_log;
use std::collections::HashMap;
use env::env::EnvType;
use ::log::LevelFilter;
use ::log::{debug, error, info, trace, warn};


fn main() {
    init_log(LevelFilter::Info);

    let user_env: Vec<(&str, &str)> = vec![
        ("EPICS_CA_ADDR_LIST", "1.2.3.4"),
        ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
    ];
    let mut env = Env::new(user_env);
    let mut context = Context {
        env: env,
    };

}
