mod context;
mod env;

use context::context::Context;
use env::env::Env;
use std::collections::HashMap;
use env::env::EnvType;

fn main() {
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
