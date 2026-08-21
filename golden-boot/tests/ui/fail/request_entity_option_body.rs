use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[request_body]
    body: Option<String>,
}

fn main() {}
