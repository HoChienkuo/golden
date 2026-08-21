use golden_boot::golden_boot_application;

#[golden_boot_application(port = 8080)]
async fn main() {
    println!("Hello, world!");
}
