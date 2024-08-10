use dbrunner::db_runner_service_server::DbRunnerService;

pub mod dbrunner {
    tonic::include_proto!("dbrunner.v1");
}

#[derive(Debug, Default)]
pub struct DbRunner {}

// #[tonic::async_trait]
// impl DbRunnerService for DbRunner {
//     async fn run_query(
//         &self,
//         request: tonic::Request<dbrunner::RunQueryRequest>,
//     ) -> Result<tonic::Response<dbrunner::RunQueryResponse>, tonic::Status> {
//     }
// }
