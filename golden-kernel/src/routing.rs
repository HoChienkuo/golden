use std::{any::Any, collections::HashMap};

use axum::Router;

use crate::ApplicationError;

#[doc(hidden)]
pub struct RouteDefinition {
    pub method: &'static str,
    pub path: &'static str,
    pub handler_name: &'static str,
    pub register:
        fn(Router, &(dyn Any + Send + Sync), &'static str) -> Result<Router, ApplicationError>,
}

inventory::collect!(RouteDefinition);

#[doc(hidden)]
pub fn validate_registered_routes() -> Result<(), ApplicationError> {
    let routes = registered_routes();
    validate_routes(&routes)
}

#[doc(hidden)]
pub fn create_router<S>(state: &S) -> Result<Router, ApplicationError>
where
    S: Any + Clone + Send + Sync + 'static,
{
    let routes = registered_routes();

    validate_routes(&routes)?;

    let state_type = std::any::type_name::<S>();
    let erased_state: &(dyn Any + Send + Sync) = state;

    routes.into_iter().try_fold(Router::new(), |router, route| {
        let router = (route.register)(router, erased_state, state_type)?;

        println!(
            "Mapped {:7} {} -> {}",
            route.method, route.path, route.handler_name,
        );

        Ok(router)
    })
}

fn registered_routes() -> Vec<&'static RouteDefinition> {
    let mut routes = inventory::iter::<RouteDefinition>
        .into_iter()
        .collect::<Vec<_>>();

    routes.sort_by_key(|route| (route.path, route.method, route.handler_name));
    routes
}

fn validate_routes(routes: &[&RouteDefinition]) -> Result<(), ApplicationError> {
    let mut registered = HashMap::<(String, String), &RouteDefinition>::new();

    for route in routes {
        let normalized_path = normalize_path(route.path);

        for effective_method in effective_methods(route.method) {
            let key = (effective_method.to_owned(), normalized_path.clone());

            if let Some(first) = registered.insert(key, route) {
                return Err(ApplicationError::DuplicateRoute {
                    method: effective_method,
                    path: route.path,
                    first_handler: first.handler_name,
                    second_handler: route.handler_name,
                });
            }
        }
    }

    Ok(())
}

fn effective_methods(method: &'static str) -> impl Iterator<Item = &'static str> {
    let methods: &'static [&'static str] = match method {
        "GET" => &["GET", "HEAD"],
        "POST" => &["POST"],
        "PUT" => &["PUT"],
        "PATCH" => &["PATCH"],
        "DELETE" => &["DELETE"],
        "HEAD" => &["HEAD"],
        "OPTIONS" => &["OPTIONS"],
        "TRACE" => &["TRACE"],
        "CONNECT" => &["CONNECT"],
        _ => &[],
    };

    methods.iter().copied()
}

fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register(
        router: Router,
        _: &(dyn Any + Send + Sync),
        _: &'static str,
    ) -> Result<Router, ApplicationError> {
        Ok(router)
    }

    #[test]
    fn rejects_equivalent_dynamic_routes() {
        let first = RouteDefinition {
            method: "GET",
            path: "/articles/{id}",
            handler_name: "first",
            register,
        };

        let second = RouteDefinition {
            method: "GET",
            path: "/articles/{name}",
            handler_name: "second",
            register,
        };

        let result = validate_routes(&[&first, &second]);

        assert!(matches!(
            result,
            Err(ApplicationError::DuplicateRoute { .. })
        ));
    }

    #[test]
    fn rejects_get_and_head_on_same_path() {
        let get = RouteDefinition {
            method: "GET",
            path: "/articles",
            handler_name: "get_articles",
            register,
        };

        let head = RouteDefinition {
            method: "HEAD",
            path: "/articles",
            handler_name: "head_articles",
            register,
        };

        let result = validate_routes(&[&get, &head]);

        assert!(matches!(
            result,
            Err(ApplicationError::DuplicateRoute { method: "HEAD", .. })
        ));
    }

    #[test]
    fn allows_different_methods_on_same_path() {
        let get = RouteDefinition {
            method: "GET",
            path: "/articles",
            handler_name: "get_articles",
            register,
        };

        let post = RouteDefinition {
            method: "POST",
            path: "/articles",
            handler_name: "create_article",
            register,
        };

        assert!(validate_routes(&[&get, &post,]).is_ok());
    }
}
