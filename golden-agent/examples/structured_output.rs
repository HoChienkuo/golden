use golden_agent::{
    ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message, ResponseFormat, ToolSchema,
};

/// The structured answer the model must produce.
///
/// `ToolSchema` derives the JSON Schema that is injected into the system
/// prompt; `serde::Deserialize` turns the model's JSON back into this type.
#[derive(Debug, serde::Deserialize, ToolSchema)]
struct WeatherReport {
    /// The city name.
    city: String,
    /// Temperature in degrees Celsius.
    temperature_c: f64,
    /// A one-word condition, e.g. "sunny".
    conditions: String,
    /// Whether it is windy.
    windy: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("set DEEPSEEK_API_KEY to run this example");
        return Ok(());
    }

    // The schema comes from the same derive the `#[tool]` attribute uses, so
    // the prompt and the target Rust type can never drift apart silently.
    let schema = WeatherReport::schema();
    let schema_text = serde_json::to_string_pretty(&schema)?;
    let system = format!(
        "You produce structured output. Reply with exactly one JSON object \
         that validates against this JSON Schema, and with no other text:\n{schema_text}"
    );

    let request = ChatRequest::new(DeepSeekModel::Flash.as_str())
        .add_message(Message::system(system))
        .add_message(Message::user("What is the weather in Taipei right now?"))
        // Instructs the provider at the decoding level, on top of the prompt
        // instruction above: output outside JSON becomes impossible. DeepSeek
        // only accepts `json_object`; `json_schema` is rejected with a 400.
        .response_format(ResponseFormat::JsonObject);

    let response = DeepSeekLlm::from_env().chat(&request).await?;

    println!("--- raw response ---\n{}", response.text());
    println!("finish_reason: {:?}", response.finish_reason);

    // One call deserializes; the error variant carries the raw text for
    // logging or a retry feed-back.
    match response.structured::<WeatherReport>() {
        Ok(report) => {
            println!("--- parsed ---");
            println!("city:         {}", report.city);
            println!("temperature_c: {}", report.temperature_c);
            println!("conditions:   {}", report.conditions);
            println!("windy:        {}", report.windy);
        }
        Err(error) => eprintln!("--- parse failed ---\n{error}"),
    }

    Ok(())
}
