use golden_agent::tool;
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
async fn main() {
    let tools = golden_agent::__private::into_chat_tools();

    println!("registered {} tool(s):", tools.len());
    for tool in &tools {
        println!("  - {}", tool.function.name);
    }

    let registered = golden_agent::__private::registered_tools();

    // Exercise get_weather.
    if let Some(weather) = registered.iter().find(|t| t.name == "get_weather") {
        let args = serde_json::json!({ "city": "Beijing", "unit": "celsius" });
        let result = (weather.call)(args).await.unwrap();
        println!("get_weather -> {result}");
    }

    // Exercise place_order with a structured argument.
    if let Some(order) = registered.iter().find(|t| t.name == "place_order") {
        let args = serde_json::json!({ "order": { "item": "coffee", "quantity": 2 } });
        let result = (order.call)(args).await.unwrap();
        println!("place_order -> {result}");
    }
}
