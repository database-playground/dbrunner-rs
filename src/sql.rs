pub mod error;
pub mod executor;
pub mod fmt;
pub mod uid;

pub use error::Error;
pub use executor::execute_query;
use serde::{Deserialize, Serialize};
pub use uid::{Hash as Blake3Hash, UidGetter};

/// A SQL query.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Query {
    /// The initial SQL (migration).
    pub initial_sql: String,

    /// The SQL query to run.
    pub query: String,
}

impl Query {
    /// Format the SQL query.
    pub fn format(self) -> Result<Query, Error> {
        let formatted_query = fmt::format_sql(&self.query)?;
        Ok(Query {
            initial_sql: self.initial_sql,
            query: formatted_query,
        })
    }
}

/// A standard SQL query response.
#[derive(Clone, Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryResponse {
    pub header: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}
