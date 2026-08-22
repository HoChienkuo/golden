use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[request_param(name = "")]
    page: u32,
}

fn main() {}
