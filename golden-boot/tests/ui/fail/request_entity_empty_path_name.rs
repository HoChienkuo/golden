use golden_boot::RequestEntity;

#[derive(RequestEntity)]
#[request_entity(rejection = Error)]
struct Request {
    #[path_variable(name = "")]
    id: u64,
}

fn main() {}
