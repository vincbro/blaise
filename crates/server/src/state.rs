use blaise::{raptor::Allocator, repository::Repository};
use crossbeam_queue::ArrayQueue;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::trace;

pub struct AppState {
    pub gtfs_data_path: PathBuf,
    pub repository: RwLock<Option<Repository>>,
    pub allocator_pool: RwLock<Option<AllocatorPool>>,
}

pub struct AllocatorPool {
    // A fixed-capacity lock-free queue
    inner: Arc<ArrayQueue<Allocator>>,
}

impl AllocatorPool {
    pub fn new(capacity: usize, repository: &Repository) -> Self {
        let queue = ArrayQueue::new(capacity);
        for _ in 0..capacity {
            let _ = queue.push(Allocator::new(repository));
        }
        Self {
            inner: Arc::new(queue),
        }
    }

    pub fn get(&self) -> Option<AllocatorGuard> {
        self.inner.pop().map(|alloc| AllocatorGuard {
            allocator: Some(alloc),
            owned: true,
            pool: self.inner.clone(),
        })
    }

    pub fn get_safe(&self, repository: &Repository) -> AllocatorGuard {
        self.inner
            .pop()
            .map(|alloc| AllocatorGuard {
                allocator: Some(alloc),
                owned: true,
                pool: self.inner.clone(),
            })
            .unwrap_or({
                trace!("Created new allocator");
                AllocatorGuard {
                    allocator: Some(Allocator::new(repository)),
                    owned: false,
                    pool: self.inner.clone(),
                }
            })
    }
}

pub struct AllocatorGuard {
    pub allocator: Option<Allocator>,
    owned: bool,
    pool: Arc<ArrayQueue<Allocator>>,
}

impl Drop for AllocatorGuard {
    fn drop(&mut self) {
        if let Some(mut alloc) = self.allocator.take()
            && self.owned
        {
            alloc.reset();
            let _ = self.pool.push(alloc);
        }
    }
}
