use golden_boot::get_mapping;

#[get_mapping("/users/{id}/articles/{id}")]
async fn get_article() {}

fn main() {}