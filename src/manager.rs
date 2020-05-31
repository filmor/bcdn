use crate::cache::Cache;
use std::collections::HashMap;

pub struct CacheManager {
    caches: HashMap<String, Cache>,
}

impl CacheManager {
    pub fn new() -> Self {
        CacheManager {
            caches: HashMap::new(),
        }
    }
}
