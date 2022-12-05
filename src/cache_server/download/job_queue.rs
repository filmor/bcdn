use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    hash::Hash,
};

pub struct JobQueue<K: Clone + Eq + Hash, D> {
    slots: Vec<Option<K>>,
    jobs: HashMap<K, Job<D>>,
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
        unimplemented!()
        /* self.jobs.entry(key).or_insert_with(|| {
            self.queue.push_back(key);
            Job::Pending { data }
        }); */
    }
    
    pub fn reset(&mut self, slot: usize) {
        
    }

    pub fn complete(&mut self, slot: usize) {
        
    }

    pub fn pop(&mut self, slot: usize) -> Option<(K, D)> {
        // TODO: Parameter: which slot is being used?
        unimplemented!()
        /* self.queue.pop_front().map(|key| {
            let mut res = None;
            self.jobs.entry(key).and_modify(|val| {
                let Job::Pending { data } = val;
                res = *val;
                *val = None;
            });
            (key, res.unwrap())
        }) */
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
