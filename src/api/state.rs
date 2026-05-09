use crate::api::store::CollectionStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub store: CollectionStore,
}
