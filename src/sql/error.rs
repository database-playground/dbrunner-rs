use tokio::task::JoinError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("format SQL: {0}")]
    Format(#[from] sql_insight::error::Error),

    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("execute initial SQL: {0}")]
    ExecuteInitialSql(rusqlite::Error),

    #[error("execute query: {0}")]
    ExecuteQuery(rusqlite::Error),

    #[error("query timed out")]
    QueryTimedOut,

    #[error("retrieve result: {0}")]
    RetrieveResult(#[from] JoinError),

    #[error("transform query result: {0}")]
    TransformQueryResult(rusqlite::Error),
}
