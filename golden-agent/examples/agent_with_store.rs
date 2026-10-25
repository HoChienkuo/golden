
use golden_agent::{
    Agent, ChatRequest, DeepSeekLlm, DeepSeekModel, InMemoryStore, Message,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    // A long-term store: `.store(...)` also registers the `remember` / `recall`
    // tools, so the model can save and look up facts on its own.
    let agent = Agent::builder()
        .model(DeepSeekLlm::from_env())
        .request(ChatRequest::new(DeepSeekModel::Flash.as_str()))
        .store(InMemoryStore::new())
        .system_prompt(
            "You have long-term memory. Use `remember` to save facts and \
             `recall` to look them up. Answer concisely.",
        )
        .build()?;

    // Round 1: tell the model something to remember.
    let result = agent
        .invoke(Message::user("My name is Alice and I prefer dark mode."))
        .await?;
    println!("round 1:\n{}\n", result.text());

    // Round 2: ask it to recall. Because the store is shared across runs (via
    // the tools), the model can `recall` what it saved in round 1.
    let result = agent
        .invoke(Message::user("What do you remember about me?"))
        .await?;
    println!("round 2:\n{}\n", result.text());

    // You can also read the store directly, outside the tools.
    if let Some(store) = agent.store() {
        let items = store.search(&vec!["users".to_string()], 0, 10).await?;
        println!("store now holds {} item(s):", items.len());
        for item in items {
            println!("  {:?} {} = {}", item.namespace, item.key, item.value);
        }
    }

    Ok(())
}
