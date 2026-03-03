use std::env;
use gen_sqlx_type::gen_sqlx_type;
use sqlx::postgres::PgPoolOptions;

gen_sqlx_type!(Task, file="queries/all_tasks.sql");

gen_sqlx_type!(GetTask, source="select * from tasks where id=$1");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new().connect(&db_url).await?;
    let all_tasks = Task::fetch_all(&pool, TaskParams {}).await?;
    for task in all_tasks {
        let task_json = serde_json::to_string(&task.clone())?;
        println!("{}", task_json);
    }

    let first_task = GetTask::fetch_one(&pool, GetTaskParams { p1: 1 }).await?;
    println!("First task: {:?}", first_task);

    Ok(())
}
