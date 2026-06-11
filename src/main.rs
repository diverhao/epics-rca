mod ca;
mod channel;
mod context;
mod env;
mod udp;

use crate::context::context::create_context;
use crate::context::context::get_context;
use ::log::LevelFilter;

#[tokio::main]
async fn main() {
    create_context(
        vec![
            ("EPICS_CA_ADDR_LIST", "192.168.3.4"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Debug,
    )
    .await;

    let context = get_context();

    println!("{:?}", context.env().get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env().get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    context.create_channel("val1");
    context.channels().search_ca().await;
}
