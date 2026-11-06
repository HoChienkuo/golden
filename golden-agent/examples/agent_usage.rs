use futures_util::StreamExt;
use golden_agent::{
    Agent, AgentEvent, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, ModelInfo,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    let llm = DeepSeekLlm::from_env();
    let model_id = DeepSeekModel::Flash.as_str();
    let model = llm
        .list_models()
        .await?
        .into_iter()
        .find(|model| model.id == model_id)
        .unwrap_or_else(|| ModelInfo::new(model_id));
    let agent = Agent::builder()
        .model(llm)
        .request(ChatRequest::new(model_id))
        .build()?;

    let mut stream = agent.stream(Message::user("What is context window size?"));
    while let Some(event) = stream.next().await {
        if let AgentEvent::Text(text) = event? {
            print!("{text}");
        }
    }

    if let Some(usage) = stream.usage() {
        println!("\n[context] {}", model.context_usage(usage));
    } else {
        println!("\n[context] usage unavailable");
    }

    // With agent.invoke(...), read the same counters from result.usage().
    Ok(())
}
