use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    hash::Hash,
};

pub struct JobQueue<K: Clone + Eq + Hash, D> {
    /// Which downloader (slot) is used by which key
    slots: Vec<Option<K>>,
    /// Collection of all job states by key
    jobs: HashMap<K, Job<D>>,
    /// Actual queue to order the loading
    queue: VecDeque<K>,
}

enum Job<D> {
    InWork { slot: usize, data: D },
    Pending { data: D },
    Done,
}

impl<K: Clone + Eq + Hash, D> JobQueue<K, D> {
    pub fn new(slots: usize) -> Self {
        JobQueue {
            slots: vec![None; slots],
            jobs: Default::default(),
            queue: Default::default(),
        }
    }

    pub fn push(&mut self, key: K, data: D) {
        // Is there already a job with this key => nothing to do (return?)
        self.jobs.entry(key.clone()).or_insert_with(|| {
            self.queue.push_back(key);
            Job::Pending { data }
        });
    }
    
    pub fn reset(&mut self, slot: usize) {
        
    }

    /// Count the given slot as completed
    pub fn complete(&mut self, slot: usize) {
        
    }

    pub fn pop(&mut self, slot: usize) -> Option<(K, D)> {
        // TODO: Parameter: which slot is being used?
        /* self.queue.pop_front().map(|key| {
            let mut res = None;
            self.jobs.entry(key).and_modify(|val| {
                let Job::Pending { data } = val;
                res = *val;
                *val = None;
            });
            (key, res.unwrap())
        }) */
        unimplemented!()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
