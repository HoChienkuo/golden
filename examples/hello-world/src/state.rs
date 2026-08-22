use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) article_service: Arc<ArticleService>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            article_service: Arc::new(ArticleService::new()),
        }
    }
}

pub(crate) struct ArticleService;

impl ArticleService {
    fn new() -> Self {
        Self
    }

    pub(crate) async fn find(&self, id: u64) -> String {
        format!("article {id}")
    }
}
