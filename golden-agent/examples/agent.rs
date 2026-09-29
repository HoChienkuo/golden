use golden_agent::{Agent, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, Retry, ToolSet, tool};

/// Get the current weather for a city.
#[tool]
async fn get_weather(city: String) -> String {
    format!("The weather in {city} is 20 degrees celsius")
}

/// Evaluate a simple arithmetic expression.
#[tool(name = "calculate", description = "Evaluate an arithmetic expression")]
async fn calculator(expression: String) -> String {
    format!("The result of `{expression}` is 42")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    // The agent drives the ReAct loop: model call, run the requested tools, feed
    // the results back, and repeat until the model answers without a tool call.
    // `Retry` adds automatic retries for transient model failures.
    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .tools(ToolSet::registered())
        .system_prompt("You answer concisely and call tools when needed.")
        .middleware(Retry::new())
        .max_steps(8)
        .build()?;

    let result = agent
        .invoke(Message::user("What's the weather in Beijing?"))
        .await?;

    for message in result.messages() {
        println!("{:?}: {}", message.role, message.text());
    }
    println!("\n{}", result.text());

    Ok(())
}
