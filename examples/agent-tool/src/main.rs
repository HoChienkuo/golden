use golden_agent::tool;
use golden_agent::{ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, Tool};
use serde::{Deserialize, Serialize};

/// Get the current weather for a city, in the given unit.
#[tool]
async fn get_weather(city: String, unit: String) -> String {
    format!("The weather in {city} is 20 degrees {unit}")
}

/// Evaluate a simple arithmetic expression.
#[tool(name = "calculate", description = "Evaluate an arithmetic expression")]
async fn calculator(expression: String) -> String {
    format!("The result of `{expression}` is 42")
}

/// A structured request demonstrating custom Deserialize parameter types.
#[derive(Deserialize, Serialize)]
struct Order {
    item: String,
    quantity: u32,
}

/// Place an order for a structured item.
#[tool]
async fn place_order(order: Order) -> String {
    format!("Ordered {} x {}", order.quantity, order.item)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    let llm = DeepSeekLlm::from_env();
    let tools = DeepSeekLlm::tools();
    let mut messages = vec![Message::user("What's the weather in Beijing?")];

    // The agent loop: send the conversation, run any tools the model asks for,
    // and repeat until it answers without requesting another tool. Today this
    // has to live in application code, because the framework does not provide it
    // yet. A production loop would also cap the number of rounds; this example
    // trusts the model to stop.
    loop {
        let request = ChatRequest::new(DeepSeekModel::Flash.as_str())
            .messages(messages.clone())
            .tools(tools.clone());

        let response = llm.chat(&request).await?;

        let Some(message) = response
            .choices
            .first()
            .map(|choice| choice.message.clone())
        else {
            eprintln!("(the model returned no choices)");
            break;
        };

        let tool_calls = message.tool_calls.clone().unwrap_or_default();
        if tool_calls.is_empty() {
            println!("{}", message.content());
            break;
        }

        // Record the assistant turn that requested the tools, then run each call
        // locally and append its result as a `tool` message for the next round.
        messages.push(Message::assistant(message.content()).tool_calls(tool_calls.clone()));

        for call in &tool_calls {
            println!("-> {}({})", call.function.name, call.function.arguments);
            let result = call_tool(&call.function.name, &call.function.arguments).await?;
            println!("<- {result}");
            messages.push(Message::tool(call.id.clone(), result));
        }
    }

    Ok(())
}

/// Looks up a registered `#[tool]` by name and invokes it with the model's
/// JSON arguments, returning its JSON-encoded result.
async fn call_tool(name: &str, arguments: &str) -> Result<String, Box<dyn std::error::Error>> {
    let tool = golden_agent::registered_tools()
        .into_iter()
        .find(|tool| tool.name == name)
        .ok_or_else(|| format!("unknown tool `{name}`"))?;

    let arguments: serde_json::Value = serde_json::from_str(arguments)?;
    Ok(tool.call(arguments).await?)
}
