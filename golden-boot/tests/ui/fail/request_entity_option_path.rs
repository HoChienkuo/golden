use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[path_variable]
    id: Option<u64>,
}

fn main() {}
