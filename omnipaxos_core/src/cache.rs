use std::hash::Hash;
use lru::LruCache;
use std::num::NonZeroUsize;
use lfu_cache::LfuCache;
use serde::ser::{Serialize, Serializer, SerializeMap};
use serde::de::{Deserialize, Deserializer, Visitor, MapAccess};
use std::fmt;
use indexmap::IndexMap;

/// The struct that is used in the cache as key
/// When an entry arrives, it needs to be converted into CacheKey
/// For SQL, this could be a string containing the extracted query template
#[derive(Debug, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct CacheKey {
    /// content
    pub val: String,
}

/// The struct that is used in the cache as value
/// When an entry arrives, it needs to be converted into CacheKey
/// For SQL, this could be a string containing the query template
#[derive(Debug, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct CacheValue {
    /// content
    pub val: String,
}

/// A helper wrapper around the cache
#[derive(Debug)]
struct SerializableLfuCache
{
    cache: LfuCache<CacheKey, CacheValue>,
}    

/// A helper wrapper around the cache
#[derive(Debug)]
struct SerializableLruCache 
{
    cache: LruCache<CacheKey, CacheValue>,
}

type IndexedCache = IndexMap<CacheKey, CacheValue>;

/// The cache we use in paxos
/// It supports operations like with(), put(key, value), get(key)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CacheModel
{
    use_lfu: bool,
    lfu_cache: SerializableLfuCache,
    lru_cache: SerializableLruCache,
    index_cache: IndexedCache,
}

impl CacheModel
{
    /// create a new cache model
    pub fn with(capacity: usize, use_lfu: bool) -> Self {
        let lfu_cache_capacity = if use_lfu { capacity } else { 1 };
        let lru_cache_capacity = if use_lfu { 1 } else { capacity };

        CacheModel {
            use_lfu,
            lfu_cache: SerializableLfuCache { cache: LfuCache::with_capacity(lfu_cache_capacity) },
            lru_cache: SerializableLruCache { cache: LruCache::new(NonZeroUsize::new(lru_cache_capacity).unwrap()) },
            index_cache: IndexMap::with_capacity(capacity),
        }
    }

    /// save (key, value) pair into cache
    /// Optimization: The value could be skipped if it always equals to the key
    pub fn put(&mut self, key: CacheKey, value: CacheValue) {
        if self.use_lfu {
            self.lfu_cache.cache.insert(key.clone(), value.clone());
        } else {
            self.lru_cache.cache.put(key.clone(), value.clone());
        }

        self.index_cache.insert(key, value);
    }

    /// get index from cache
    pub fn get_index_of(&mut self, key: CacheKey) -> Option<usize> {
        if self.use_lfu {
            if let Some(_value) = self.lfu_cache.cache.get(&key) {
                return self.index_cache.get_index_of(&key)
            }
        } else {
            if let Some(_value) = self.lru_cache.cache.get(&key) {
                return self.index_cache.get_index_of(&key)
            } 
        }

        None
    }

    /// get value from cache
    pub fn get_with_index(&mut self, index: usize) -> Option<(&CacheKey, &CacheValue)> {
        self.index_cache.get_index(index)
    }

    /// return cache length
    pub fn len(&self) -> usize {
        if self.use_lfu {
            self.lfu_cache.cache.len()
        } else {
            self.lru_cache.cache.len()
        }
    }
}

/// Serialization functions
impl Serialize for SerializableLfuCache
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.cache.len()))?;
        for (k, v) in self.cache.peek_iter() {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

impl Serialize for SerializableLruCache
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.cache.len()))?;
        for (k, v) in self.cache.iter() {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
    }
}

/// Deserialization functions
struct LfuCacheVisitor {}
impl<'de> Visitor<'de> for LfuCacheVisitor
{
    // The type that our Visitor is going to produce.
    type Value = SerializableLfuCache;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("expects lfu cache")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = SerializableLfuCache { 
            cache: LfuCache::with_capacity(access.size_hint().unwrap_or(0))
        };

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry()? {
            map.cache.insert(key, value);
        }

        Ok(map)
    }
}

impl<'de> Deserialize<'de> for SerializableLfuCache
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LfuCacheVisitor {})
    }
}
struct LruCacheVisitor {}
impl<'de> Visitor<'de> for LruCacheVisitor
{
    // The type that our Visitor is going to produce.
    type Value = SerializableLruCache;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("expects lru cache")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut map = SerializableLruCache { 
            cache: LruCache::new(NonZeroUsize::new(access.size_hint().unwrap_or(1)).unwrap())
        };

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry()? {
            map.cache.put(key, value);
        }

        Ok(map)
    }
}

impl<'de> Deserialize<'de> for SerializableLruCache
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LruCacheVisitor {})
    }
}