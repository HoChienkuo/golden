use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    page: u32,
}

fn main() {}
