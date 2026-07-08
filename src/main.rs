#![allow(dead_code, unused_imports, unused_variables)]

mod alloc_stats;
mod ca;
mod channel;
mod context;
mod env;
mod tcp;
mod udp;

use crate::channel::channel::Channel;
use crate::channel::dbr::ChannelState;
use crate::channel::dbr::DbrType;
use crate::channel::dbr::DbrValue;
use crate::channel::dbr_data::DbrData;
use crate::channel::monitor::MonitorDataType;
use crate::channel::monitor::MonitorState;
use crate::context::context::create_context;
use crate::context::context::get_context;
use ::log::LevelFilter;
use ::log::debug;
use std::fmt::format;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::{self};
use tokio::time::{Duration, sleep};

#[tokio::main(flavor = "current_thread")]
// #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    create_context(
        vec![
            ("EPICS_CA_ADDR_LIST", "127.0.0.1"),
            ("EPICS_CA_AUTO_ADDR_LIST", "NO"),
        ],
        LevelFilter::Error,
    )
    .await;

    let context = get_context();
    context.start_search_ca();

    println!("{:?}", context.env().get_env("EPICS_CA_BEACON_PERIOD"));
    println!(
        "{:?}",
        context.env().get_env_source("EPICS_CA_BEACON_PERIODaaa")
    );

    // tokio::spawn(async move {
    //     let mut interval = time::interval(Duration::from_millis(1000));

    //     loop {
    //         interval.tick().await;

    //         // periodic work here
    //         let mut monitor_running_count = 0;
    //         let mut name_found_count = 0;
    //         let mut tcp_connected_count = 0;
    //         let mut created_count = 0;
    //         let mut waiting_on_tcp_channels = 0;
    //         for channel in get_context().channels().not_searching_by_cid().values() {
    //             match channel.state() {
    //                 ChannelState::NameFound => name_found_count += 1,
    //                 ChannelState::TcpConnected => tcp_connected_count += 1,
    //                 ChannelState::Created => created_count += 1,
    //                 _ => {}
    //             }
    //             if channel.monitor().state() == MonitorState::Running {
    //                 monitor_running_count += 1;
    //             }
    //         }
    //         println!(
    //             "not sesarching {}, monitor running {}, name found  {}, tcp connected {}, created {}, waiting on tcp {}, no wait on tcp {}, self connect {}",
    //             get_context().channels().not_searching_by_cid().len(),
    //             monitor_running_count,
    //             name_found_count,
    //             tcp_connected_count,
    //             created_count,
    //             get_context()
    //                 .tcps()
    //                 .wait_connected_count
    //                 .load(Ordering::Relaxed),
    //             get_context()
    //                 .tcps()
    //                 .already_connected_count
    //                 .load(Ordering::Relaxed),
    //             get_context()
    //                 .tcps()
    //                 .self_connect_count
    //                 .load(Ordering::Relaxed)
    //         );
    //     }
    // });

    // let data = Arc::new(RwLock::new(0.0));
    // let data_for_callback = Arc::clone(&data);

    // let channel1 = context.create_channel("val1");
    // let channel5 = context.create_channel("val5");
    // println!("{}", context.channels());

    // channel.get(channel::dbr::DbrType::StsDouble, 1).await;
    // channel1.get(Some(5.0), None, None, None).await;

    let callback1 = Arc::new(
        move |cid: u32, data_type: DbrType, data_count: u32, dbr_data: &DbrData| {
            // prinxtln!("{}", dbr_data);
            // println!(">> {}", channel.name());
            // debug!(
            //     "{} has a new value: {:?}, {}",
            //     channel.name(),
            //     channel.value(),
            //     channel.meta()
            // );
            // let value = match channel.value().clone().unwrap() {
            //     DbrValue::Double(value) => Some(value),
            //     _ => None,
            // };
            // if let Some(value) = value {
            //     // *data_for_callback.write().unwrap() = value[0];
            //     debug!("{:?}", value);
            // }
            // if let Some(data) = channel.dbr_data(channel.dbr_type_native_as_time()) {
            //     debug!("------------------------------>>>{}", data);
            // }
        },
    );
    // let callback5 = move |channel: &Channel| {
    //     debug!(
    //         "{} has a new value: {:?}, {}",
    //         channel.name(),
    //         channel.value(),
    //         channel.meta()
    //     );
    //     let value = match channel.value().clone().unwrap() {
    //         DbrValue::Double(value) => Some(value),
    //         _ => None,
    //     };
    //     if let Some(value) = value {
    //         // *data_for_callback.write().unwrap() = value[0];
    //         debug!("{:?}", value);
    //     }
    // };

    // let callback_1a = Arc::clone(&callback1);
    // channel1
    //     .start_to_monitor(
    //         Some(MonitorDataType::NativeCtrl),
    //         None,
    //         Some(callback_1a),
    //     )
    //     .await;
    // println!("+++++++++++++++++++++++++++++++++++++++++++++++");
    // channel5
    //     .start_to_monitor(
    //         Some(channel1.dbr_type_native_as_gr()),
    //         None,
    //         Some(Arc::new(callback5)),
    //     )
    //     .await;
    // sleep(Duration::from_secs(50)).await;
    // channel1.destroy().await;

    // debug!(
    //     "-----------> {:?}, {}, {}, {}",
    //     channel.value(),
    //     data.read().unwrap(),
    //     channel.monitor(),
    //     channel
    // );
    // context.create_channel("val2afadsfsa");
    // println!("{}", context.channels());

    for ii in 0..100000 {
        let callback = Arc::clone(&callback1);
        // println!("{}", ii);
        let context = get_context().clone();
        // tokio::spawn(async move {
        let name = format!("val{}", ii);
        // println!("{}", name);
        let channel = context.create_channel(&name);
        channel.start_to_monitor(Some(MonitorDataType::NativeTime), None, Some(callback));
        // println!("-->{}", name);
        // });
    }

    print_alloc_stats("after-create-loop");

    let mut alloc_report_interval = time::interval(Duration::from_secs(5));
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = alloc_report_interval.tick() => {
                print_alloc_stats("periodic");
            }
            result = &mut ctrl_c => {
                result.expect("failed to listen for Ctrl-C");
                break;
            }
        }
    }

    print_alloc_stats("final");
}

fn print_alloc_stats(label: &str) {
    let stats = alloc_stats::snapshot();
    println!(
        "ALLOC_STATS {label}: requested current {:.3} MiB, peak {:.3} MiB, peak-current {:.3} MiB; usable current {:.3} MiB, peak {:.3} MiB; calls alloc {}, dealloc {}, realloc {}",
        alloc_stats::mib(stats.current_bytes),
        alloc_stats::mib(stats.peak_bytes),
        alloc_stats::mib(stats.peak_bytes.saturating_sub(stats.current_bytes)),
        alloc_stats::mib(stats.current_usable_bytes),
        alloc_stats::mib(stats.peak_usable_bytes),
        stats.alloc_count,
        stats.dealloc_count,
        stats.realloc_count,
    );
}
