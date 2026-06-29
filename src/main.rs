#![allow(dead_code, unused_imports, unused_variables)]

mod ca;
mod channel;
mod context;
mod env;
mod tcp;
mod udp;

use crate::channel::channel::Channel;
use crate::channel::dbr::DbrValue;
use crate::context::context::create_context;
use crate::context::context::get_context;
use ::log::LevelFilter;
use ::log::debug;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::{Duration, sleep};

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

    let data = Arc::new(RwLock::new(0.0));
    let data_for_callback = Arc::clone(&data);

    let channel = context.create_channel("val1");
    // channel.get(channel::dbr::DbrType::StsDouble, 1).await;
    channel.get(None, None).await;

    let callback = move |channel: &Channel| {
        debug!("{} has a new value: {:?}, {}", channel.name(), channel.value(), channel.meta());
        let value = match channel.value().clone().unwrap() {
            DbrValue::Double(value) => Some(value),
            _ => None,
        };
        if let Some(value) = value {
            *data_for_callback.write().unwrap() = value[0];
            debug!("{:?}", value);
        }
    };

    channel
        .start_to_monitor(Some(channel.dbr_type_native_to_gr()), None, Some(Arc::new(callback)))
        .await;
    sleep(Duration::from_secs(5)).await;
    channel.cancel_monitor().await;


    debug!("-----------> {:?}, {}, {}", channel.value(), data.read().unwrap(), channel.monitor());
    // context.create_channel("val2afadsfsa");
    println!("{}", context.channels());
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl-C");
}
