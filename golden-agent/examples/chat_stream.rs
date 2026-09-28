use futures_util::StreamExt;
use golden_agent::{ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = DeepSeekLlm::from_env();

    let request = ChatRequest::new(DeepSeekModel::Flash.as_str()).add_message(Message::user(
        "Write a short poem about Rust. Keep it under 50 words.",
    ));

    let mut stream = llm.chat_stream(&request);

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(text) => print!("{text}"),
            Err(e) => eprintln!("\nstream error: {e}"),
        }
    }
    println!();

    Ok(())
}
