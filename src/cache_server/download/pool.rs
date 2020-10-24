use std::path::PathBuf;

use super::super::cache::Cache;
use crate::Config;

pub struct DownloadPool;

unsafe impl Send for DownloadPool {}
unsafe impl Sync for DownloadPool {}

impl DownloadPool {
    pub fn new(config: &Config) -> Self {
        DownloadPool
    }

    pub fn enqueue(&self, cache: &Cache, filename: &str) -> DownloadState {
        DownloadState {
            downloaded: 0,
            size: 0,
        }
    }
}

pub struct DownloadState {
    downloaded: u64,
    size: u64,
}

impl DownloadState {
    pub fn is_done(&self) -> bool {
        self.downloaded >= self.size
    }
    
    pub fn percentage(&self) -> u8 {
        if self.size == 0 {
            100
        } else {
        ((self.downloaded as f32) / (self.size as f32)) as u8
        }
    }
}
