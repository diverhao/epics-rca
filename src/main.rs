#![allow(dead_code, unused_imports, unused_variables)]

mod ca;
mod channel;
mod context;
mod env;
mod tcp;
mod udp;

use crate::context::context::create_context;
use crate::context::context::get_context;
use ::log::LevelFilter;

#[tokio::main]
async fn main() {
    create_context(
        vec![
            ("EPICS_CA_ADDR_LIST", "127.0.0.1"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Debug,
    )
    .await;

    let context = get_context();
    context.start_search_ca();

    println!("{:?}", context.env().get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env().get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    let channel = context.create_channel("val1");
    channel.get(channel::dbr::DbrType::StsDouble, 1).await;
    // context.create_channel("val2afadsfsa");
    println!("{}", context.channels());
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");
}
