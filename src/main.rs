mod channel;
mod context;
mod env;
mod udp;

use std::sync::Mutex;

use ::log::LevelFilter;
use context::context::CONTEXT;
use context::context::Context;
use crate::context::context::create_context;
use crate::context::context::get_context;


fn main() {
    create_context(
        vec![
            ("EPICS_CA_ADDR_LIST", "1.2.3.4"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Info,
    );

    let context: std::sync::MutexGuard<'_, Context> = get_context();

    println!("{:?}", context.env.get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env.get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    // let mut channel = channel::channel::Channel::new("ABCD");
    // println!("{}", channel);
    // channel.name = "AAAA".to_string();
    // println!("{:?}", context.env.get_env("EPICS_CA_ADDR_LIST"));
}
