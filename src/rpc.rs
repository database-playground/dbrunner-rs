use std::{pin::Pin, result::Result};

pub use dbrunner::db_runner_service_server::{DbRunnerService, DbRunnerServiceServer};
use dbrunner::{
    retrieve_query_response::Kind, run_query_response::ResponseType, AreQueriesOutputSameRequest,
    AreQueriesOutputSameResponse, Cell, DataRow, HeaderRow, RetrieveQueryRequest,
    RetrieveQueryResponse, RunQueryResponse,
};
use tokio_stream::{iter as stream_iter, Stream};
use tonic::{Request, Response, Status};

use crate::{
    cache,
    sql::{self, Query, QueryResponse, UidGetter},
};

pub mod dbrunner {
    tonic::include_proto!("dbrunner.v1");
}

#[derive(Debug)]
pub struct DbRunner {
    redis_client: redis::Client,
}

impl DbRunner {
    pub fn new(redis_client: redis::Client) -> Self {
        Self { redis_client }
    }

    async fn redis_conn(&self) -> Result<redis::aio::MultiplexedConnection, Status> {
        let client = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| Status::internal(format!("Failed to get Redis connection: {e}")))?;

        Ok(client)
    }
}

#[tonic::async_trait]
impl DbRunnerService for DbRunner {
    type RetrieveQueryStream =
        Pin<Box<dyn Stream<Item = Result<RetrieveQueryResponse, Status>> + Send + Sync>>;

    async fn run_query(
        &self,
        request: Request<dbrunner::RunQueryRequest>,
    ) -> Result<Response<RunQueryResponse>, Status> {
        let (_, _, data) = request.into_parts();

        let mut conn = self.redis_conn().await?;
        let mut cacher = cache::RedisCacher::new(&mut conn);

        let query = Query {
            initial_sql: data.schema,
            query: data.query,
        }
        .format()
        .map_err(|e| Status::invalid_argument(format!("Invalid query: {e}")))?;

        let query_uid = query.get_uid().to_hex();

        // Return the cache if it exists.
        if let Ok(cache::CacheState::Hit(_)) = cacher.get::<QueryResponse>(query_uid.as_str()).await
        {
            return Ok(Response::new(RunQueryResponse {
                response_type: Some(ResponseType::Id(query_uid.to_string())),
            }));
        }

        // Run the query.
        match sql::execute_query(query).await {
            Ok(response) => {
                // Store the response in the cache.
                cacher
                    .set(&query_uid, response)
                    .await
                    .map_err(|e| Status::internal(format!("Failed to set cache: {e}")))?;

                Ok(Response::new(RunQueryResponse {
                    response_type: Some(ResponseType::Id(query_uid.to_string())),
                }))
            }
            Err(e) => match e {
                sql::Error::ExecuteInitialSql(_)
                | sql::Error::ExecuteQuery(_)
                | sql::Error::QueryTimedOut
                | sql::Error::TransformQueryResult(_) => Ok(Response::new(RunQueryResponse {
                    response_type: Some(ResponseType::Error(e.to_string())),
                })),
                _ => Err(Status::internal(format!("Failed to run query: {e}"))),
            },
        }
    }

    async fn retrieve_query(
        &self,
        request: Request<RetrieveQueryRequest>,
    ) -> Result<Response<Self::RetrieveQueryStream>, Status> {
        let data = request.get_ref();

        let mut conn = self.redis_conn().await?;
        let mut cacher = cache::RedisCacher::new(&mut conn);

        let query_uid = data.id.as_str();

        let response = match cacher.get::<QueryResponse>(query_uid).await {
            Ok(cache::CacheState::Hit(response)) => response,
            Ok(cache::CacheState::Miss) => {
                return Err(Status::not_found(format!(
                    "Query with ID {} not found. Run RunQuery again?",
                    query_uid
                )))
            }
            Err(e) => return Err(Status::internal(format!("Failed to get cache: {e}"))),
        };

        // map the response to a RetrieveQueryResponse stream
        let query_responses = Box::pin(stream_iter(
            itertools::chain![
                std::iter::once(RetrieveQueryResponse {
                    kind: Some(Kind::Header(HeaderRow {
                        cells: response.header,
                    })),
                }),
                response.rows.into_iter().map(|row| RetrieveQueryResponse {
                    kind: Some(Kind::Row(DataRow {
                        cells: row.into_iter().map(|r| Cell { value: r }).collect(),
                    })),
                })
            ]
            .map(Ok::<_, Status>),
        )) as Self::RetrieveQueryStream;

        Ok(Response::new(query_responses))
    }

    async fn are_queries_output_same(
        &self,
        request: Request<AreQueriesOutputSameRequest>,
    ) -> Result<Response<AreQueriesOutputSameResponse>, Status> {
        let data = request.get_ref();
        let (left, right) = (data.left_id.as_str(), data.right_id.as_str());

        let mut conn = self.redis_conn().await?;
        let mut cacher = cache::RedisCacher::new(&mut conn);

        cacher
            .same_output_uid(left, right)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "Failed to check if queries have the same output: {e}"
                ))
            })
            .map(|same| Response::new(AreQueriesOutputSameResponse { same }))
    }
}
