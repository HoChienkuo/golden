use futures_util::StreamExt;
use golden_agent::{
    Agent, AgentEvent, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, ToolSet, tool,
};

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

    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .tools(ToolSet::registered())
        .system_prompt("You answer concisely and call tools when needed.")
        .max_steps(8)
        .build()?;

    // Streaming surfaces text as it is generated, and reports each tool call and
    // its result as the ReAct loop runs.
    let mut stream = agent.stream(Message::user("What's the weather in Beijing?"));

    while let Some(event) = stream.next().await {
        match event? {
            AgentEvent::Text(text) => print!("{text}"),
            AgentEvent::ToolCall(call) => println!("\n[tool] {}({})", call.name, call.arguments),
            AgentEvent::ToolResult { call, content } => {
                println!("[result] {} -> {content}", call.name);
            }
            AgentEvent::Done => println!(),
        }
    }

    Ok(())
}
