use std::env;
use std::str::FromStr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use gen_sqlx_type::gen_sqlx_type;

gen_sqlx_type!(Task, file="queries/all_tasks.sql");
gen_sqlx_type!(NoSerdeTask, file="queries/all_tasks.sql", serde=false);
gen_sqlx_type!(NoCloneTask, file="queries/all_tasks.sql", clone=false);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    let all_tasks = Task::fetch_all(&pool, TaskParams {}).await?;
    for task in all_tasks {
        println!("Task: {:?}", task.clone());
        let _json = serde_json::to_string(&task)?;
    }

    let _no_serde = NoSerdeTask {
        id: 1,
        title: "test".to_string(),
        description: None,
        status: "pending".to_string(),
        created_at: None,
        updated_at: None,
    };
    // The following would fail to compile if I uncommented them:
    // let _json = serde_json::to_string(&_no_serde)?; 

    let _no_clone = NoCloneTask {
        id: 1,
        title: "test".to_string(),
        description: None,
        status: "pending".to_string(),
        created_at: None,
        updated_at: None,
    };
    // let _cloned = _no_clone.clone(); // This should fail if clone=false works

    Ok(())
}
