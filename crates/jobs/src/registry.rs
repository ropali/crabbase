use std::{collections::HashMap, sync::Arc};

use crate::handler::JobHandler;

#[derive(Clone, Default)]
pub struct Registry {
    handlers: HashMap<&'static str, Arc<dyn JobHandler>>,
}

impl Registry {
    pub fn new(handlers: Vec<Arc<dyn JobHandler>>) -> Self {
        let mut map = HashMap::new();
        for handler in handlers {
            map.insert(handler.kind(), handler);
        }
        Self { handlers: map }
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(kind).cloned()
    }
}
