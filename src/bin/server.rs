//! mini-redis server.
//!
//! This file is the entry point for the server implemented in the library. It
//! performs command line parsing and passes the arguments on to
//! `mini_redis::server`.
//!
//! The `clap` crate is used for parsing arguments.

use mini_redis::{server, DEFAULT_PORT};

use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber::prelude::*;

use std::alloc::System;
use tracking_allocator::Allocator;

#[global_allocator]
static GLOBAL: Allocator<System> = Allocator::system();

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> mini_redis::Result<()> {
    // enable logging
    // see https://docs.rs/tracing for more info
    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(tracking_allocator::AllocationLayer::new())
        .with(tracing_subscriber::fmt::layer())
        .init();

    tokio::spawn(trace_allocations());

    let cli = Cli::from_args();
    let port = cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    // Bind a TCP listener
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

    server::run(listener, signal::ctrl_c()).await;

    Ok(())
}

pub async fn trace_allocations() {
    use tracking_allocator::{AllocationTracker, AllocationRegistry, AllocationGroupId};

    #[derive(Debug)]
    enum AllocationEvent {
        Allocated { addr: usize, size: usize },
        Deallocated { addr: usize },
    }

    struct ChannelBackedTracker {
        sender: std::sync::mpsc::SyncSender<AllocationEvent>,
    }

    impl AllocationTracker for ChannelBackedTracker {
        fn allocated(&self, addr: usize, size: usize, _: AllocationGroupId) {
            let _ = self.sender.try_send(AllocationEvent::Allocated {
                addr, size,
            });
        }

        fn deallocated(&self, addr: usize, _: AllocationGroupId) {
            let _ = self.sender.try_send(AllocationEvent::Deallocated {
                addr,
            });
        }
    }

    let (tx, rx) = std::sync::mpsc::sync_channel(10000);

    let tracker = ChannelBackedTracker {
        sender: tx,
    };

    let _ = AllocationRegistry::set_global_tracker(tracker)
        .expect("no other global tracker should be set yet");
        
    AllocationRegistry::enable_tracking();

    tokio::task::spawn_blocking(move || {
        loop {
            match rx.recv() {
                Ok(AllocationEvent::Allocated { addr, size, .. }) => {
                    tracing::trace!{
                        addr = addr,
                        size = size,
                        "alloc"
                    };
                },
                Ok(AllocationEvent::Deallocated { addr, .. }) => {
                    tracing::trace!{
                        addr = addr,
                        "dealloc"
                    };
                },
                _ => {},
            }
        }
    });   
}

#[derive(StructOpt, Debug)]
#[structopt(name = "mini-redis-server", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "A Redis server")]
struct Cli {
    #[structopt(name = "port", long = "--port")]
    port: Option<String>,
}
