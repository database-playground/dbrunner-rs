#![feature(assert_matches)]

use tonic::transport::Server;

pub mod cache;
pub mod rpc;
pub mod sql;

#[tokio::main]
async fn main() {
    let addr = std::env::var("ADDR")
        .or_else(|_| match std::env::var("PORT") {
            Ok(port) => Ok(format!("0.0.0.0:{}", port)),
            Err(e) => Err(e),
        })
        .unwrap_or("127.0.0.1:50051".to_string())
        .parse()
        .expect("Failed to parse address");

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
