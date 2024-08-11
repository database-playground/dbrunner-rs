use super::{Query, QueryResponse};
pub use blake3::Hash;
use std::io::Write;

/// A trait to get the UID of a query (for caching).
pub trait UidGetter {
    /// Get the UID of this query (for caching).
    ///
    /// You should normalize it (by formatting, etc.) before running this method.
    fn get_uid(&self) -> Hash;
}

impl UidGetter for Query {
    /// Get the UID of this query (for caching).
    ///
    /// You should [format] it before running this method.
    fn get_uid(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.initial_sql.as_bytes());
        hasher.update("\x00".as_bytes());
        hasher.update(self.query.as_bytes());
        hasher.finalize()
    }
}

impl UidGetter for QueryResponse {
    /// Get the UID of this query (for caching).
    fn get_uid(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();

        write!(hasher, "{:?}", self.header).unwrap();
        hasher.update("\x00".as_bytes());
        write!(hasher, "{:?}", self.rows).unwrap();

        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_query() {
        let query_a1 = Query {
            initial_sql: "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
                .to_string(),
            query: "SELECT * FROM test".to_string(),
        };
        let query_a2 = Query {
            initial_sql: "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
                .to_string(),
            query: "SELECT * FROM test".to_string(),
        };
        let query_b = Query {
            initial_sql: "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
                .to_string(),
            query: "SELECT * FROM test WHERE id = 1".to_string(),
        };
        let query_b2 = Query {
            initial_sql: "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
                .to_string(),
            query: "select * from test where id = 1".to_string(),
        }
        .format()
        .expect("should formattable");
        let query_c = Query {
            initial_sql: "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)"
                .to_string(),
            query: "SELECT * FROM test WHERE id = 2".to_string(),
        };

        assert_eq!(query_a1.get_uid(), query_a2.get_uid());
        assert_ne!(query_a1.get_uid(), query_b.get_uid());
        assert_ne!(query_a1.get_uid(), query_c.get_uid());
        assert_ne!(query_b.get_uid(), query_c.get_uid());
        assert_eq!(query_b.get_uid(), query_b2.get_uid());
    }

    #[test]
    fn test_hash_result() {
        let response_a1 = QueryResponse {
            header: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Some("1".to_string()), Some("Alice".to_string())]],
        };
        let response_a2 = QueryResponse {
            header: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Some("1".to_string()), Some("Alice".to_string())]],
        };
        let response_b = QueryResponse {
            header: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Some("2".to_string()), Some("Bob".to_string())]],
        };

        assert_eq!(response_a1.get_uid(), response_a2.get_uid());
        assert_ne!(response_a1.get_uid(), response_b.get_uid());
        assert_ne!(response_a2.get_uid(), response_b.get_uid());
    }
}
