use crate::raster::DiagramPixmap;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagramCacheKey {
    pub content_hash: u64,
    pub is_dark: bool,
    pub scale_quantized: u16,
}

impl DiagramCacheKey {
    pub fn new(source: &str, is_dark: bool, scale: f32) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let content_hash = hasher.finish();

        let scale_quantized = (scale * 100.0).round().clamp(10.0, 5000.0) as u16;

        Self {
            content_hash,
            is_dark,
            scale_quantized,
        }
    }
}

pub struct DiagramCache {
    capacity: usize,
    map: HashMap<DiagramCacheKey, (DiagramPixmap, u64)>,
    access_counter: u64,
}

impl DiagramCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            access_counter: 0,
        }
    }

    pub fn get(&mut self, key: &DiagramCacheKey) -> Option<&DiagramPixmap> {
        self.access_counter += 1;
        let counter = self.access_counter;
        if let Some(entry) = self.map.get_mut(key) {
            entry.1 = counter;
            Some(&entry.0)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: DiagramCacheKey, pixmap: DiagramPixmap) {
        self.access_counter += 1;
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            // Evict least recently used (lowest access counter)
            if let Some(oldest_key) = self
                .map
                .iter()
                .min_by_key(|(_, (_, access))| *access)
                .map(|(k, _)| k.clone())
            {
                self.map.remove(&oldest_key);
            }
        }
        self.map.insert(key, (pixmap, self.access_counter));
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insertion_and_lru() {
        let mut cache = DiagramCache::new(2);
        let key1 = DiagramCacheKey::new("graph 1", false, 1.0);
        let key2 = DiagramCacheKey::new("graph 2", false, 1.0);
        let key3 = DiagramCacheKey::new("graph 3", false, 1.0);

        let p1 = DiagramPixmap {
            width: 10,
            height: 10,
            premultiplied_rgba: vec![0; 400],
        };
        let p2 = DiagramPixmap {
            width: 20,
            height: 20,
            premultiplied_rgba: vec![0; 1600],
        };
        let p3 = DiagramPixmap {
            width: 30,
            height: 30,
            premultiplied_rgba: vec![0; 3600],
        };

        cache.insert(key1.clone(), p1);
        cache.insert(key2.clone(), p2);
        assert!(cache.get(&key1).is_some());

        // Inserting 3rd entry should evict key2 (since key1 was accessed more recently)
        cache.insert(key3.clone(), p3);
        assert!(cache.get(&key1).is_some());
        assert!(cache.get(&key2).is_none());
        assert!(cache.get(&key3).is_some());
    }
}
