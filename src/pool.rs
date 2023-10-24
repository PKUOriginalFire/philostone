use std::collections::HashSet;

use async_lock::RwLock;
use heapless::HistoryBuffer;
use slotmap::{new_key_type, SlotMap};

use crate::Danmaku;

new_key_type! {
    /// The default slot map key type.
    pub struct PoolKey;
}

/// A pool of danmaku, containing the last N danmaku.
pub struct DanmakuPool<const N: usize> {
    storage: SlotMap<PoolKey, Danmaku>,
    history: Box<HistoryBuffer<PoolKey, N>>,
}

impl<const N: usize> DanmakuPool<N> {
    /// Creates a new danmaku pool.
    pub fn new() -> Self {
        DanmakuPool {
            storage: SlotMap::with_capacity_and_key(N),
            history: Box::new(HistoryBuffer::new()),
        }
    }

    /// Inserts a danmaku into the pool.
    pub fn insert(&mut self, danmaku: Danmaku) -> PoolKey {
        let key = self.storage.insert(danmaku);
        self.history.write(key);
        key
    }

    /// Gets a danmaku from the pool.
    pub fn get(&self, key: PoolKey) -> Option<&Danmaku> {
        self.storage.get(key)
    }

    /// Iterates over the danmaku in the pool, from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &Danmaku> {
        self.history.oldest_ordered().map(|key| &self.storage[*key])
    }

    /// Performs garbage collection on the pool.
    pub fn garbage_collect(&mut self) {
        let keys = self.history.iter().copied().collect::<HashSet<_>>();
        self.storage.retain(|key, _| keys.contains(&key));
    }
}

impl Default for DanmakuPool<100> {
    fn default() -> Self {
        Self::new()
    }
}

pub type DefaultPool = RwLock<DanmakuPool<100>>;
