use std::sync::{atomic::AtomicUsize, Arc, Mutex};

pub struct IdGenerator {
    next_id: AtomicUsize,
    free_ids: Arc<Mutex<Vec<usize>>>,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
            free_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_next_id(&mut self) -> usize {
        if let Some(id) = self.free_ids.lock().unwrap().pop() {
            id
        } else {
            let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if id == usize::MAX {
                panic!("ID overflow");
            }
            id
        }
    }

    pub fn release_id(&mut self, id: usize) {
        self.free_ids.lock().unwrap().push(id);
    }
}
