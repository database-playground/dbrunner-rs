#![feature(assert_matches)]

use std::net::SocketAddr;

use mimalloc_rust::GlobalMiMalloc;
use tonic::transport::Server;

pub mod cache;
pub mod rpc;
pub mod sql;

#[global_allocator]
static GLOBAL_MIMALLOC: GlobalMiMalloc = GlobalMiMalloc;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .unwrap_or("3000".to_string())
        .parse()
        .expect("PORT must be a number");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let redis_addr = std::env::var("REDIS_ADDR").expect("REDIS_ADDR must be set");
    let redis_client = redis::Client::open(redis_addr).expect("Failed to open Redis client");

    let dbrunner_service = rpc::DbRunner::new(redis_client);
    println!("Server listening on {}", addr);

    Server::builder()
        .add_service(rpc::DbRunnerServiceServer::new(dbrunner_service))
        .serve(addr)
        .await
        .expect("Failed to serve");
}
