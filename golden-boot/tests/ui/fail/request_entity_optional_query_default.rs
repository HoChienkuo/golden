use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[request_param(default = 1)]
    page: Option<u32>,
}

fn main() {}
