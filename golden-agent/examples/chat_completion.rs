use golden_agent::{ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = DeepSeekLlm::from_env();

    let request = ChatRequest::new(DeepSeekModel::Flash.as_str())
        .add_message(Message::user("What is 1 + 1? Just give the answer."));
    let response = llm.chat(&request).await?;
    println!("{}", response.text());
    Ok(())
}
