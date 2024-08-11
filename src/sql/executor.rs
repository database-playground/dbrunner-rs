use std::time::Duration;

use rusqlite::types::Value;

use super::{Error, Query, QueryResponse};

pub async fn execute_query(query: Query) -> Result<QueryResponse, Error> {
    let formatted_query = query.format()?;

    let handle = tokio::task::spawn_blocking(move || {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.busy_timeout(Duration::from_secs(3))?;

        // run the initial SQL
        conn.execute_batch(&formatted_query.initial_sql)
            .map_err(Error::ExecuteInitialSql)?;

        // run the query
        let mut stmt = conn
            .prepare(&formatted_query.query)
            .map_err(Error::ExecuteQuery)?;
        let column_count = stmt.column_count();
        let rows = stmt
            .query_map((), |row| {
                let mut row_data = Vec::with_capacity(column_count);
                for i in 0..column_count {
                    let cell = row.get::<_, Value>(i)?;
                    match cell {
                        Value::Null => row_data.push(None),
                        Value::Integer(i) => row_data.push(Some(i.to_string())),
                        Value::Real(f) => row_data.push(Some(f.to_string())),
                        Value::Text(s) => row_data.push(Some(s)),
                        Value::Blob(b) => {
                            row_data.push(Some(String::from_utf8_lossy(&b).to_string()))
                        }
                    }
                }
                Ok(row_data)
            })?
            .collect::<Result<Vec<Vec<Option<String>>>, rusqlite::Error>>()
            .or_else(|e| match e {
                rusqlite::Error::SqliteFailure(_, Some(error_message))
                    if error_message == "not an error" =>
                {
                    Ok(Vec::new())
                }
                _ => Err(Error::TransformQueryResult(e)),
            })?;
        let header = stmt
            .column_names()
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>();

        Ok::<_, Error>(QueryResponse { header, rows })
    });
    let timeout_result = tokio::time::timeout(Duration::from_secs(5), handle).await;

    match timeout_result {
        Err(e) => Err(Error::QueryTimedOut(e)),
        Ok(Err(e)) => Err(Error::RetrieveResult(e)),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Ok(Ok(response))) => Ok(response),
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;

    #[tokio::test]
    async fn test_with_valid_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "SELECT * FROM test;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(response.header, vec!["id", "name"]);
        assert_eq!(
            response.rows,
            vec![
                vec![Some("1".to_string()), Some("Alice".to_string())],
                vec![Some("2".to_string()), Some("Bob".to_string())]
            ]
        );
    }

    #[tokio::test]
    async fn test_with_no_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(response.header.len(), 0, "header should be empty");
        assert_eq!(response.rows.len(), 0, "rows should be empty");
    }

    #[tokio::test]
    async fn test_with_update_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "UPDATE test SET name = 'Charlie' WHERE id = 1;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(response.header.len(), 0, "header should be empty");
        assert_eq!(response.rows.len(), 0, "rows should be empty");
    }

    #[tokio::test]
    async fn test_with_update_returning_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "UPDATE test SET name = 'Charlie' WHERE id = 1 RETURNING *;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(response.header, vec!["id", "name"]);
        assert_eq!(
            response.rows,
            vec![vec![Some("1".to_string()), Some("Charlie".to_string())]]
        );
    }

    #[test]
    fn test_with_dos_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: r#"
                WITH RECURSIVE cte (n) AS (
                    SELECT 1
                    UNION ALL
                    SELECT n + 1 FROM cte
                )
                SELECT * FROM cte;
            "#
            .to_string(),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime build");
        let response = rt.block_on(execute_query(query));

        assert_matches!(response, Err(Error::QueryTimedOut(_)));
        // kill runtime
        rt.shutdown_background();
    }

    #[tokio::test]
    async fn test_with_malformed_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "SELECT * FROM unknown_table;".to_string(),
        };
        let response = execute_query(query).await;

        assert_matches!(response, Err(Error::ExecuteQuery(_)));
    }

    #[tokio::test]
    async fn test_with_invalid_query() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test (name) VALUES ('Alice');
                INSERT INTO test (name) VALUES ('Bob');
            "#
            .to_string(),
            query: "SELECT * FROM test WHERE id = ':D)D)D))D)D)D)D)D;".to_string(),
        };
        let response = execute_query(query).await;
        assert_matches!(response, Err(Error::Format(_)));
    }

    #[tokio::test]
    async fn test_with_invalid_schema() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                ABCDEFG;
                "#
            .to_string(),
            query: "SELECT * FROM test;".to_string(),
        };
        let response = execute_query(query).await;
        assert_matches!(response, Err(Error::ExecuteInitialSql(_)));
    }

    #[tokio::test]
    async fn test_with_nil_return() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test VALUES (1, NULL);
                "#
            .to_string(),
            query: "SELECT * FROM test;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(
            response.rows,
            vec![vec![Some("1".to_string()), None]],
            "cell should be <nil>"
        );
    }

    #[tokio::test]
    async fn test_with_real_number() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test VALUES (1, 1.23);
                "#
            .to_string(),
            query: "SELECT * FROM test;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(
            response.rows,
            vec![vec![Some("1".to_string()), Some("1.23".to_string())]],
            "cell should be 1.23"
        );
    }

    #[tokio::test]
    async fn test_with_blob() {
        let query = Query {
            initial_sql: r#"
                CREATE TABLE test (
                    id INTEGER PRIMARY KEY,
                    name TEXT
                );

                INSERT INTO test VALUES (1, x'68656c6c6f');
                "#
            .to_string(),
            query: "SELECT * FROM test;".to_string(),
        };
        let response = execute_query(query).await.expect("no error");
        assert_eq!(
            response.rows,
            vec![vec![Some("1".to_string()), Some("hello".to_string())]],
            "cell should be 'hello'"
        );
    }
}
