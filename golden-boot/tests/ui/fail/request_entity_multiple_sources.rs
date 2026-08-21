use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[path_variable]
    #[request_param]
    id: u64,
}

fn main() {}
