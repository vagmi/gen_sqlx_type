use std::env;
use std::str::FromStr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use gen_sqlx_type::gen_sqlx_type;

gen_sqlx_type!(Task, file="queries/all_tasks.sql");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    let all_tasks = sqlx::query_file_as!(Task, "queries/all_tasks.sql").fetch_all(&pool).await?;
    for task in all_tasks {
        println!("Task: {:?}", task);
    }
    Ok(())
}
