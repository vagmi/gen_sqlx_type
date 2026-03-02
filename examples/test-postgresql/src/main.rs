use std::env;
use gen_sqlx_type::gen_sqlx_type;
use sqlx::postgres::PgPoolOptions;

gen_sqlx_type!(Task, file="queries/all_tasks.sql");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().connect(&db_url).await?;
    let all_tasks = sqlx::query_file_as!(Task, "queries/all_tasks.sql").fetch_all(&pool).await?;
    for task in all_tasks {
        let task_json = serde_json::to_string(&task.clone())?;
        println!("{}", task_json);
    }
    Ok(())
}
