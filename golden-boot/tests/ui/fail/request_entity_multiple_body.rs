use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[request_body]
    first: String,

    #[request_body]
    second: String,
}

fn main() {}
