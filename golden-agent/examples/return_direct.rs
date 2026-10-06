use golden_agent::{Agent, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, ToolSet, tool};

/// Calculate `a <operator> b` for the operators `+`, `-`, `*`, and `/`.
///
/// The tool is declared `return_direct`, so its result becomes the agent's
/// final answer: the run ends right after the tool executes, without another
/// model round to rephrase the number.
#[tool(return_direct)]
async fn calculate(
    a: f64,
    #[param(description = "One of +, -, *, /")] operator: String,
    b: f64,
) -> String {
    match operator.as_str() {
        "+" => (a + b).to_string(),
        "-" => (a - b).to_string(),
        "*" => (a * b).to_string(),
        "/" if b != 0.0 => (a / b).to_string(),
        "/" => "error: cannot divide by zero".to_string(),
        other => format!("error: unknown operator `{other}`"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    // The agent calls the model, runs the requested tool, and stops: because
    // `calculate` is `return_direct`, its result is echoed as the last
    // assistant message instead of being fed back for another model round.
    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .tools(ToolSet::registered()?)
        .system_prompt("You solve arithmetic by calling the calculate tool.")
        .max_steps(4)
        .build()?;

    let result = agent.invoke(Message::user("What is 12 * 3?")).await?;

    for message in result.messages() {
        println!("{:?}: {}", message.role, message.text());
    }
    println!("\nfinal answer: {}", result.text());

    Ok(())
}
