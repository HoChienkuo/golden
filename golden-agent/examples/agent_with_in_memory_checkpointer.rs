use golden_agent::{
    Agent, ChatRequest, Context, DeepSeekLlm, DeepSeekModel, InMemoryCheckpointer, Message,
    ThreadId,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }
    let cp = InMemoryCheckpointer::new();

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

    Ok(())
}
