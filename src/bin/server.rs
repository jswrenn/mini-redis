//! mini-redis server.
//!
//! This file is the entry point for the server implemented in the library. It
//! performs command line parsing and passes the arguments on to
//! `mini_redis::server`.
//!
//! The `clap` crate is used for parsing arguments.

use mini_redis::{server, DEFAULT_PORT};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_cloudwatch::{Client as CWClient, Region};
use structopt::StructOpt;
use tokio::net::TcpListener;
use tokio::signal;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> mini_redis::Result<()> {
    // enable logging
    // see https://docs.rs/tracing for more info
    tracing_subscriber::fmt::try_init()?;

    let cli = Cli::from_args();
    let port = cli.port.as_deref().unwrap_or(DEFAULT_PORT);

    // Bind a TCP listener
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

    // Construct a CloudWatch client
    let region_provider = RegionProviderChain::first_try(cli.region.map(Region::new))
        .or_default_provider()
        .or_else(Region::new("us-west-2"));

    let shared_config = aws_config::from_env().region(region_provider).load().await;
    let cw_client = CWClient::new(&shared_config);

    server::run(listener, cw_client, signal::ctrl_c()).await;

    Ok(())
}

#[derive(StructOpt, Debug)]
#[structopt(name = "mini-redis-server", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "A Redis server")]
struct Cli {
    #[structopt(name = "port", long = "--port")]
    port: Option<String>,

    /// The AWS Region.
    #[structopt(short, long)]
    region: Option<String>,
}
