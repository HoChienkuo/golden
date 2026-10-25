use golden_agent::{ChatModel, ChatRequest, Message, OllamaLlm};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = OllamaLlm::new();

    let request = ChatRequest::new("llama3.1")
        .add_message(Message::user("What is 1 + 1? Just give the answer."));
    let response = llm.chat(&request).await?;
    println!("{}", response.text());
    Ok(())
}
