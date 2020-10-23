use crate::Config;

pub struct DownloadPool;

unsafe impl Send for DownloadPool {}
unsafe impl Sync for DownloadPool {}

impl DownloadPool {
    pub fn new(config:&Config) -> Self {
        DownloadPool
    }
}