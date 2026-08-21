use golden_boot::RequestEntity;

#[derive(RequestEntity)]
struct Request {
    #[request_param]
    page: u32,
}

fn main() {}
