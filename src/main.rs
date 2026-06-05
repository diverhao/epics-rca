mod channel;
mod context;
mod env;
mod udp;

use crate::context::context::{create_context, get_runtime};
use ::log::LevelFilter;

#[tokio::main]
async fn main() {
    create_context(
        vec![
            ("EPICS_CA_ADDR_LIST", "1.2.3.4"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Info,
    )
    .await;

    let runtime = get_runtime();
    let context = runtime.context();

    println!("{:?}", context.env.get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env.get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    context.udp.send_ca(
        &runtime,
        udp::udp::CaCmd::CaProtoBuild,
        22,
        channel::channel::DbrType::Char,
        33,
        22,
        55,
    );

    // let mut channel = channel::channel::Channel::new("ABCD");
    // println!("{}", channel);
    // channel.name = "AAAA".to_string();
    // println!("{:?}", context.env.get_env("EPICS_CA_ADDR_LIST"));
}
