use crate::alloc_stats;
use crate::ca_message::header::CaHeader;
use crate::ca_message::message::CaMsg;
use crate::ca_channel::ca_channel::{CaChannel, ChannelCallback};
use crate::ca_channel::ca_channels::{CaChannels, ChannelIo};
use crate::ca_channel::dbr::{DbrType, DbrValue};
use crate::ca_channel::ca_meta::CaMeta;
use crate::ca_channel::ca_monitor::{CaMonitor, MonitorConfig, MonitorData, MonitorDataType};
use crate::context::context::get_context;
use crate::spawn_stats;
use crate::tcp::tcp::{TCP, TCPs};
use crate::udp::udp::UDP;
use std::collections::{HashMap, VecDeque};
use std::mem::{align_of, size_of};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool as StdAtomicBool, Ordering};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, RwLock};
use tokio::sync::{Mutex, Notify};

pub fn print_memory_report(label: &str) {
    println!("---- memory report: {label} ----");
    print_type_sizes_once();
    print_allocator_stats();
    print_spawn_stats();
    print_traffic_stats();
    print_container_stats();
    println!("---- end memory report: {label} ----");
}

fn print_type_sizes_once() {
    static PRINTED: StdAtomicBool = StdAtomicBool::new(false);
    if !PRINTED.swap(true, Ordering::Relaxed) {
        print_type_sizes();
    }
}

fn print_type_sizes() {
    println!("type sizes:");
    print_type::<CaChannel>("CaChannel");
    print_type::<CaMeta>("CaMeta");
    print_type::<CaMonitor>("CaMonitor");
    print_type::<MonitorData>("MonitorData");
    print_type::<MonitorConfig>("MonitorConfig");
    print_type::<TCP>("TCP");
    print_type::<TCPs>("TCPs");
    print_type::<UDP>("UDP");
    print_type::<CaChannels>("CaChannels");
    print_type::<ChannelIo>("ChannelIo");
    print_type::<CaMsg>("CaMsg");
    print_type::<CaHeader>("CaHeader");
    print_type::<DbrType>("DbrType");
    print_type::<DbrValue>("DbrValue");
    print_type::<MonitorDataType>("MonitorDataType");
    print_type::<Arc<CaChannel>>("Arc<CaChannel>");
    print_type::<ChannelCallback>("ChannelCallback");
    print_type::<String>("String");
    print_type::<Box<str>>("Box<str>");
    print_type::<Vec<u8>>("Vec<u8>");
    print_type::<Vec<SocketAddr>>("Vec<SocketAddr>");
    print_type::<VecDeque<CaMsg>>("VecDeque<CaMsg>");
    print_type::<HashMap<u32, Arc<CaChannel>>>("HashMap<u32, Arc<CaChannel>>");
    print_type::<HashMap<String, Arc<CaChannel>>>("HashMap<String, Arc<CaChannel>>");
    print_type::<SocketAddr>("SocketAddr");
    print_type::<RwLock<CaMeta>>("RwLock<CaMeta>");
    print_type::<RwLock<Option<SocketAddr>>>("RwLock<Option<SocketAddr>>");
    print_type::<RwLock<Vec<u32>>>("RwLock<Vec<u32>>");
    print_type::<Mutex<Option<()>>>("tokio::Mutex<Option<()>>");
    print_type::<Notify>("Notify");
    print_type::<AtomicU32>("AtomicU32");
    print_type::<AtomicBool>("AtomicBool");
}

fn print_type<T>(name: &str) {
    println!(
        "  {:34} size {:4} align {}",
        name,
        size_of::<T>(),
        align_of::<T>()
    );
}

fn print_allocator_stats() {
    let alloc = alloc_stats::snapshot();
    println!(
        "allocator requested: current {:.3} MiB, peak {:.3} MiB, peak-current {:.3} MiB",
        alloc_stats::mib(alloc.current_bytes),
        alloc_stats::mib(alloc.peak_bytes),
        alloc_stats::mib(alloc.peak_bytes.saturating_sub(alloc.current_bytes))
    );
    println!(
        "allocator usable: current {:.3} MiB, peak {:.3} MiB, peak-current {:.3} MiB",
        alloc_stats::mib(alloc.current_usable_bytes),
        alloc_stats::mib(alloc.peak_usable_bytes),
        alloc_stats::mib(
            alloc
                .peak_usable_bytes
                .saturating_sub(alloc.current_usable_bytes)
        )
    );
    println!(
        "allocator calls: alloc {}, dealloc {}, realloc {}, total allocated {:.3} MiB, total deallocated {:.3} MiB",
        alloc.alloc_count,
        alloc.dealloc_count,
        alloc.realloc_count,
        alloc_stats::mib(alloc.total_allocated_bytes),
        alloc_stats::mib(alloc.total_deallocated_bytes)
    );
}

fn print_spawn_stats() {
    let spawn = spawn_stats::snapshot();
    println!(
        "spawns: total {}, udp_v4 {}, udp_v6 {}, search {}, tcp_write {}, tcp_read {}, tcp_alive {}, channel_connect {}, channel_reconnect {}",
        spawn.total,
        spawn.udp_v4_listen,
        spawn.udp_v6_listen,
        spawn.search_ca_loop,
        spawn.tcp_write_loop,
        spawn.tcp_read_loop,
        spawn.tcp_check_alive_loop,
        spawn.channel_connect,
        spawn.channel_reconnect
    );
}

fn print_traffic_stats() {
    let traffic = crate::traffic_stats::snapshot();
    println!(
        "traffic builds: version {}, name_search {}, create_chan {}, event_add {}, other {}, built payload {:.3} MiB",
        traffic.build_version,
        traffic.build_name_search,
        traffic.build_create_chan,
        traffic.build_event_add,
        traffic.build_other,
        alloc_stats::mib(traffic.built_payload_bytes)
    );
    println!(
        "traffic buffers: to_buf calls {}, to_buf bytes {:.3} MiB, from_buf msgs {}, from_buf payload {:.3} MiB",
        traffic.msg_to_buf_calls,
        alloc_stats::mib(traffic.msg_to_buf_bytes),
        traffic.from_buf_msgs,
        alloc_stats::mib(traffic.from_buf_payload_bytes)
    );
    println!(
        "traffic peaks: search batch len/cap {}/{}, tcp queue len/cap {}/{}, tcp drain len/cap {}/{}, tcp write cap {:.3} MiB, udp write cap {} bytes",
        traffic.search_batch_max_len,
        traffic.search_batch_max_capacity,
        traffic.tcp_queue_max_len,
        traffic.tcp_queue_max_capacity,
        traffic.tcp_drain_max_len,
        traffic.tcp_drain_max_capacity,
        alloc_stats::mib(traffic.tcp_write_buf_max_capacity),
        traffic.udp_write_buf_max_capacity
    );
}

fn print_container_stats() {
    let context = get_context();
    let channel_stats = context.ca_channels().capacity_stats();
    let tcp_stats = context.ca_tcps().capacity_stats();

    println!(
        "channels maps: searching_by_name len/cap {}/{}, searching_by_cid {}/{}, not_searching_by_cid {}/{}, ios {}/{}",
        channel_stats.searching_by_name_len,
        channel_stats.searching_by_name_capacity,
        channel_stats.searching_by_cid_len,
        channel_stats.searching_by_cid_capacity,
        channel_stats.not_searching_by_cid_len,
        channel_stats.not_searching_by_cid_capacity,
        channel_stats.ios_len,
        channel_stats.ios_capacity
    );
    println!(
        "channels map approximate bucket bytes: {:.3} MiB",
        alloc_stats::mib(channel_stats.approx_bucket_bytes())
    );

    println!(
        "tcps: connected len/cap {}/{}, connecting len/cap {}/{}, total cids len/cap {}/{}, total queues len/cap {}/{}",
        tcp_stats.tcps_len,
        tcp_stats.tcps_capacity,
        tcp_stats.connecting_tcps_len,
        tcp_stats.connecting_tcps_capacity,
        tcp_stats.total_cids_len,
        tcp_stats.total_cids_capacity,
        tcp_stats.total_queue_len,
        tcp_stats.total_queue_capacity
    );
    println!(
        "tcp approximate retained vec/deque bytes: cids {:.3} MiB, queues {:.3} MiB",
        alloc_stats::mib(tcp_stats.approx_cids_bytes()),
        alloc_stats::mib(tcp_stats.approx_queue_bytes())
    );
}
