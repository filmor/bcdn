use std::path::Path;
use url::Url;

pub struct Cache {
    name: String,
    base: Url,
    path: Box<Path>,
    // items
}
