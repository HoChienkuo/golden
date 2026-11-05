use golden_agent::DeepSeekLlm;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm = DeepSeekLlm::from_env();
    let models = llm.list_models().await?;

    for model in models {
        println!("{model:#?}");
    }

    Ok(())
}
