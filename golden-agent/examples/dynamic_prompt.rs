use golden_agent::{
    Agent, AgentState, ChatRequest, Context, DeepSeekLlm, DeepSeekModel, DynamicPrompt, Message,
};

/// Supplied through the run context, not part of the conversation.
struct LocalTime(String);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    let mut context = Context::new();
    context.insert(LocalTime("2026-09-29 09:30 (Asia/Shanghai)".to_string()));

    // `DynamicPrompt` runs before every model call, so the system prompt can
    // depend on the run context instead of being fixed when the agent is built.
    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .middleware(DynamicPrompt::new(|state: &AgentState| {
            let time = state
                .context
                .get::<LocalTime>()
                .map_or("unknown", |time| time.0.as_str());
            format!("You are a helpful assistant. The current local time is {time}.")
        }))
        .build()?;

    let result = agent
        .invoke_with_context(Message::user("What time is it?"), context)
        .await?;

    println!("{}", result.text());

    Ok(())
}
