//! Thinking mode on DeepSeek: switch it on, pick an effort level, and read the
//! reasoning the model produced.
//!
//! Run with `DEEPSEEK_API_KEY` set:
//!
//! ```text
//! cargo run -p golden-agent --example reasoning
//! ```

use futures_util::StreamExt;
use golden_agent::{
    ChatEvent, ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, Reasoning,
    ReasoningEffort,
};

const QUESTION: &str = "A bat and a ball cost $1.10 in total. \
                        The bat costs $1.00 more than the ball. \
                        How much does the ball cost?";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    let llm = DeepSeekLlm::from_env();
    let model = DeepSeekModel::Flash.as_str();

    // 1. Non-streaming: thinking is on with the strongest effort. Read the
    //    chain of thought from `reasoning()` and the answer from `text()`.
    let request = ChatRequest::new(model)
        .add_message(Message::user(QUESTION))
        .reasoning(Reasoning::Effort(ReasoningEffort::Max));

    let response = llm.chat(&request).await?;

    println!("=== thinking ===");
    println!("{}", response.reasoning());
    println!("\n=== answer ===");
    println!("{}", response.text());

    if let Some(usage) = &response.usage {
        println!(
            "\n=== usage ===\nprompt: {}  completion: {}  reasoning: {}",
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.reasoning_tokens.unwrap_or(0),
        );
    }

    // 2. Streaming: `ChatEvent::Reasoning` deltas arrive before the answer, so
    //    a UI can show the model thinking in real time.
    let request = DeepSeekLlm::thinking(ChatRequest::new(model), true)
        .reasoning(Reasoning::Effort(ReasoningEffort::Low))
        .add_message(Message::user(
            "In one sentence, what is the capital of France?",
        ));

    println!("\n=== streaming ===");
    let mut stream = llm.chat_stream(&request);
    while let Some(event) = stream.next().await {
        match event? {
            ChatEvent::Reasoning(thinking) => print!("\x1b[2m{thinking}\x1b[0m"),
            ChatEvent::ReasoningSignature(_) | ChatEvent::ContentPart(_) => {}
            ChatEvent::Text(text) => print!("{text}"),
            ChatEvent::ToolCall(call) => println!("[tool] {}({})", call.name, call.arguments),
            _ => {}
        }
    }
    println!();

    // 3. Thinking off: the same question is answered directly. `reasoning()`
    //    is empty because the model does not think. Note that DeepSeek thinks
    //    by default, so a request with no `reasoning` set is *thinking on*.
    let request =
        DeepSeekLlm::thinking(ChatRequest::new(model), false).add_message(Message::user(QUESTION));

    let response = llm.chat(&request).await?;
    println!("\n=== thinking disabled ===");
    println!("{}", response.text());
    debug_assert!(response.reasoning().is_empty());

    Ok(())
}
