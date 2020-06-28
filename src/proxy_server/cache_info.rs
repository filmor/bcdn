use crate::config::Config;
use globset::GlobSet;
use url::Url;

pub struct CacheInfo {
    nodes: Vec<NodeCacheInfo>,

    name: String,
    patterns: GlobSet,
}

pub struct NodeCacheInfo {
    url: Url,
}

impl NodeCacheInfo {
    fn new(url: Url, config: &Config) -> Self {
        NodeCacheInfo { url }
    }
}

impl CacheInfo {
    pub fn new(name: &str, config: &Config) -> Self {
        let nodes = config
            .proxy
            .nodes
            .iter()
            .map(|n| NodeCacheInfo::new(Url::parse(n).unwrap(), config))
            .collect();

        let entry = &config.entries[name];
        let patterns = entry.get_globset().unwrap();
        let name = name.to_owned();

        CacheInfo {
            name,
            nodes,
            patterns,
        }
    }

    pub fn get_redirect(&self, filename: &str) -> Option<Url> {
        if !self.patterns.is_match(filename) {
            return None;
        }

        use rand::distributions::{Distribution, Uniform};

        let dist = Uniform::from(0..self.nodes.len());
        let mut rng = rand::thread_rng();

        let node = &self.nodes[dist.sample(&mut rng)];

        let path = format!("c/v1/{}/f/{}", self.name, filename);

        Some(node.url.join(&path).unwrap())
    }
}
