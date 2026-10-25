use golden_agent::{ChatModel, ChatRequest, LlamaCppLlm, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = LlamaCppLlm::new("http://ip:port/v1");

    let request = ChatRequest::new("your model name")
        .add_message(Message::user("What is 1 + 1? Just give the answer."));
    let response = llm.chat(&request).await?;
    println!("{}", response.text());
    Ok(())
}
