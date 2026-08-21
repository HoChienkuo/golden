use golden_boot::get_mapping;

#[get_mapping("/articles/:id")]
async fn get_article() {}

fn main() {}