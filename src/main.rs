mod context;
mod env;
mod channel;

use ::log::LevelFilter;
use context::context::Context;

fn main() {
    let mut context = Context::new(
        vec![
            ("EPICS_CA_ADDR_LIST", "1.2.3.4"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Info,
    );

    println!("{:?}", context.env.get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env.get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    let mut channel = channel::channel::Channel::new("ABCD");
    println!("{}", channel);
    channel.name = "AAAA".to_string();
    // println!("{:?}", context.env.get_env("EPICS_CA_ADDR_LIST"));
}
