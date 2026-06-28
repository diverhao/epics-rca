#![allow(dead_code, unused_imports, unused_variables)]

mod ca;
mod channel;
mod context;
mod env;
mod udp;
mod tcp;

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

    // context.udp().start_to_listen();

    println!("{:?}", context.env().get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env().get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    context.create_channel("val1");
    // context.create_channel("val2afadsfsa");
    context.start_search_ca().await;
    println!("{}", context.channels());
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");
}
