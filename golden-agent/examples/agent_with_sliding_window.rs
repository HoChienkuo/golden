use golden_agent::{
    Agent, ChatRequest, Context, DeepSeekLlm, DeepSeekModel, InMemoryCheckpointer, Message,
    SlidingWindow, ThreadId,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cp = InMemoryCheckpointer::new();

    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .checkpointer(cp)
        .system_prompt("You answer concisely.")
        .middleware(SlidingWindow::new(3)) // Keep only the 3 most recent complete turns.
        .build()?;

    let mut ctx = Context::new();
    ctx.insert(ThreadId("thread-1".to_string()));

    let result = agent
        .invoke_with_context(Message::user("Hello"), ctx)
        .await?;
    println!("{}", result.text());
    Ok(())
}
