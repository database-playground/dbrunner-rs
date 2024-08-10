//! Format and Run SQL.

use sql_insight::sqlparser::dialect::PostgreSqlDialect;

pub fn format_sql(sql: &str) -> Result<String, Error> {
    let dialect = PostgreSqlDialect {};
    let formatted_sql = sql_insight::format(&dialect, sql)?;

    Ok(formatted_sql.join("; "))
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("SQL Error: {0}")]
    SqlError(#[from] sql_insight::error::Error),
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use crate::sql::format_sql;

    #[rstest]
    #[case("SELECT * FROM students", "SELECT * FROM students")]
    #[case(
        "SELECT * FROM students WHERE id = 1",
        "SELECT * FROM students WHERE id = 1"
    )]
    #[case(
        "SELECT * FROM students WHERE id = 1; -- comment",
        "SELECT * FROM students WHERE id = 1"
    )]
    #[case(
        "SELECT * FROM students WHERE id = 1; -- comment\n",
        "SELECT * FROM students WHERE id = 1"
    )]
    #[case(
        "SELECT *, aaa FROM students WHERE id = 1; -- comment\n",
        "SELECT *, aaa FROM students WHERE id = 1"
    )]
    #[case(
        "SELECT * FROM students;\nSELECT * FROM teachers;",
        "SELECT * FROM students; SELECT * FROM teachers"
    )]
    #[case("SELECT *     FROM   students", "SELECT * FROM students")]
    #[case("seLect * fRom students", "SELECT * FROM students")]
    fn test_format(#[case] input: &str, #[case] expected: &str) {
        let formatted = format_sql(input).unwrap();
        assert_eq!(
            *expected, formatted,
            "Case {input}: Expected '{expected}', got '{formatted}'"
        );
    }
}
