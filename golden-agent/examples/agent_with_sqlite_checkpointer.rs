use golden_agent::{
    Agent, ChatRequest, Context, DeepSeekLlm, DeepSeekModel, Message, SqliteCheckpointer, ThreadId,
};
use sqlx::{Row, SqlitePool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }
    // create chat.db if not exists
    let pool = SqlitePool::connect("sqlite:chat.db?mode=rwc").await?;
    let cp = SqliteCheckpointer::new(pool.clone());
    cp.migrate().await?;

    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .checkpointer(cp)
        .system_prompt("You answer concisely and call tools when needed.")
        .build()?;

    let mut ctx = Context::new();
    ctx.insert(ThreadId("thread-1".to_string()));

    let result = agent
        .invoke_with_context(Message::user("Hello"), ctx)
        .await?;
    println!("\n{}", result.text());

    let rows = sqlx::query("SELECT * FROM messages ORDER BY seq")
        .fetch_all(&pool)
        .await?;

    for row in rows {
        let id: String = row.try_get("id")?;
        let thread_id: String = row.try_get("thread_id")?;
        let run_id: String = row.try_get("run_id")?;
        let seq: i64 = row.try_get("seq")?;
        let role: String = row.try_get("role")?;
        let content: String = row.try_get("content")?;
        let is_context: i64 = row.try_get("is_context")?;

        println!(
            "[{thread_id} #{seq}] {role} (run={run_id}, ctx={is_context})\n  id={id}\n  content={content}"
        );
    }

    Ok(())
}
