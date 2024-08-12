//! Allow caching the given query in the cache.

use std::fmt::Display;

use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

use crate::sql::UidGetter;

const EXPIRE_SECONDS: u64 = 60 * 60;

const DBRUNNER_CACHER_KEY: &str = "dbrunner:cacher";

pub struct RedisCacher<'a, C: AsyncCommands> {
    conn: &'a mut C,
}

impl<'a, C: AsyncCommands> RedisCacher<'a, C> {
    pub fn new(conn: &'a mut C) -> Self {
        Self { conn }
    }

    /// Get the stored data from the cache.
    pub async fn get<T: DeserializeOwned>(
        &mut self,
        query_uid: &str,
    ) -> Result<CacheState<T>, Error> {
        use CacheState::*;

        let input_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Input,
            uid = query_uid
        );
        let Some(output_uid): Option<String> = self
            .conn
            .get_ex(&input_key, redis::Expiry::EX(EXPIRE_SECONDS))
            .await?
        else {
            return Ok(Miss);
        };

        let output_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Output,
            uid = output_uid,
        );
        let Some(output): Option<String> = self
            .conn
            .get_ex(&output_key, redis::Expiry::EX(EXPIRE_SECONDS))
            .await?
        else {
            return Ok(Miss);
        };

        let output = serde_json::from_str(&output).unwrap();
        Ok(Hit(output))
    }

    /// Check if the two query_uid has the same output UID.
    pub async fn same_output_uid(
        &mut self,
        left_query_uid: &str,
        right_query_uid: &str,
    ) -> Result<bool, Error> {
        let left_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Input,
            uid = left_query_uid,
        );
        let right_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Input,
            uid = right_query_uid,
        );

        let (left_output_uid, right_output_uid) = redis::pipe()
            .get_ex(&left_key, redis::Expiry::EX(EXPIRE_SECONDS))
            .get_ex(&right_key, redis::Expiry::EX(EXPIRE_SECONDS))
            .query_async::<(String, String)>(self.conn)
            .await?;

        Ok(left_output_uid == right_output_uid)
    }

    /// Store the data in the cache.
    pub async fn set(
        &mut self,
        query_uid: &str,
        output: impl UidGetter + Serialize,
    ) -> Result<(), Error> {
        let input_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Input,
            uid = query_uid,
        );
        let output_key = format!(
            "{key}:{kind}:{uid}",
            key = DBRUNNER_CACHER_KEY,
            kind = Kind::Output,
            uid = output.get_uid(),
        );
        let output_json = serde_json::to_string(&output).unwrap();

        redis::pipe()
            .set_ex(output_key, output_json, EXPIRE_SECONDS)
            .ignore()
            .set_ex(
                input_key,
                output.get_uid().to_hex().as_ref(),
                EXPIRE_SECONDS,
            )
            .ignore()
            .exec_async(self.conn)
            .await?;

        Ok(())
    }
}

pub enum Kind {
    Input,
    Output,
}

impl Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Input => write!(f, "input"),
            Kind::Output => write!(f, "output"),
        }
    }
}

/// The state of the cache.
pub enum CacheState<T: DeserializeOwned> {
    /// The cache hit, and the output is returned.
    Hit(T),
    /// The cache miss, and the output is not returned.
    Miss,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
}

#[cfg(test)]
mod tests {
    use crate::sql::{Query, QueryResponse, UidGetter};

    use super::RedisCacher;

    #[tokio::test]
    #[ignore = "requires a running Redis server"]
    async fn test_cache() {
        let mut conn = create_connection(0).await;
        let mut cacher = RedisCacher::new(&mut conn);

        let input = Query {
            initial_sql: "CREATE TABLE test (id INT); INSERT INTO test VALUES (1);".to_string(),
            query: "SELECT * FROM test".to_string(),
        }
        .format()
        .expect("formatting query");
        let output = QueryResponse {
            header: vec![("id".to_string())],
            rows: vec![vec![Some("1".to_string())]],
        };
        let output_c = output.clone();
        let input_uid = input.get_uid();

        cacher
            .set(input_uid.to_hex().as_str(), output)
            .await
            .expect("setting cache");

        let result = cacher
            .get::<QueryResponse>(input_uid.to_hex().as_str())
            .await
            .expect("getting cache");
        assert!(matches!(result, super::CacheState::Hit(v) if v == output_c));
    }

    #[tokio::test]
    #[ignore = "requires a running Redis server"]
    async fn test_cache_not_hit() {
        let mut conn = create_connection(1).await;
        let mut cacher = RedisCacher::new(&mut conn);

        let input = Query {
            initial_sql: "CREATE TABLE test (id INT); INSERT INTO test VALUES (1);".to_string(),
            query: "SELECT * FROM test".to_string(),
        }
        .format()
        .expect("formatting query");

        let result = cacher
            .get::<QueryResponse>(input.get_uid().to_hex().as_str())
            .await
            .expect("getting cache");
        assert!(matches!(result, super::CacheState::Miss));
    }

    #[tokio::test]
    #[ignore = "requires a running Redis server"]
    async fn test_two_uid_same() {
        let mut conn = create_connection(2).await;
        let mut cacher = RedisCacher::new(&mut conn);

        let input_a1 = Query {
            initial_sql: "CREATE TABLE test (id INT); INSERT INTO test VALUES (1);".to_string(),
            query: "SELECT * FROM test".to_string(),
        };

        let input_a2 = Query {
            initial_sql: "CREATE TABLE test (id INT); INSERT INTO test VALUES (1);".to_string(),
            query: "select * from test".to_string(),
        };

        let input_b = Query {
            initial_sql: "CREATE TABLE test (id INT); INSERT INTO test VALUES (1);".to_string(),
            query: "SELECT * FROM test WHERE id = 114514".to_string(),
        };

        let output_a = QueryResponse {
            header: vec![("id".to_string())],
            rows: vec![vec![Some("1".to_string())]],
        };

        let output_b = QueryResponse {
            header: Default::default(),
            rows: Default::default(),
        };

        // register to cache
        cacher
            .set(input_a1.get_uid().to_hex().as_str(), output_a.clone())
            .await
            .expect("setting cache");
        cacher
            .set(input_a2.get_uid().to_hex().as_str(), output_a)
            .await
            .expect("setting cache");
        cacher
            .set(input_b.get_uid().to_hex().as_str(), output_b)
            .await
            .expect("setting cache");

        // check if the two query_uid has the same output UID
        let result = cacher
            .same_output_uid(
                input_a1.get_uid().to_hex().as_str(),
                input_a2.get_uid().to_hex().as_str(),
            )
            .await
            .expect("checking same output UID");
        assert!(
            result,
            "input_a1->output_uid != input_a2->output_uid, expected =="
        );

        let result = cacher
            .same_output_uid(
                input_a1.get_uid().to_hex().as_str(),
                input_b.get_uid().to_hex().as_str(),
            )
            .await
            .expect("checking same output UID");
        assert!(
            !result,
            "input_a1->output_uid == input_b->output_uid, expected !="
        );

        let result = cacher
            .same_output_uid(
                input_a2.get_uid().to_hex().as_str(),
                input_b.get_uid().to_hex().as_str(),
            )
            .await
            .expect("checking same output UID");
        assert!(
            !result,
            "input_a2->output_uid == input_b->output_uid, expected !="
        );
    }

    async fn create_connection(test_id: u16) -> redis::aio::MultiplexedConnection {
        let integration_uri =
            std::env::var("REDIS_INTEGRATION_URI").expect("REDIS_INTEGRATION_URI is not set");
        let integration_test_db = test_id + 8;
        let redis_url = format!(
            "{uri}/{db}",
            uri = integration_uri,
            db = integration_test_db
        );

        let client = redis::Client::open(redis_url).unwrap();
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .expect("connecting to redis");

        // refresh the cache
        redis::cmd("FLUSHDB")
            .exec_async(&mut connection)
            .await
            .unwrap();

        connection
    }
}
