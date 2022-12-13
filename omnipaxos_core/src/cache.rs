use std::hash::Hash;
use lru::LruCache;
use std::num::NonZeroUsize;
use lfu_cache::LfuCache;
use serde::ser::{Serialize, Serializer, SerializeMap};
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};
use std::fmt;
use std::marker::PhantomData;

#[allow(missing_docs)]
#[typetag::serde]
pub trait Key {} 

#[allow(missing_docs)]
pub trait SerializableKey: Key + Eq + Hash + Clone {}

#[allow(missing_docs)]
#[typetag::serde]
pub trait Value {}

#[allow(missing_docs)]
pub trait SerializableValue: Value + Clone {}

#[allow(missing_docs)]
#[derive(Debug)]
struct SerializableLfuCache<K: SerializableKey, V: SerializableValue> 
{
    cache: LfuCache<K, V>,
}    

#[allow(missing_docs)]
#[derive(Debug)]
struct SerializableLruCache<K: SerializableKey, V: SerializableValue> 
{
    cache: LruCache<K, V>,
}

#[allow(missing_docs)]
#[derive(Debug, serde::Serialize)]
pub struct CacheModel<K, V>
where
    K: SerializableKey,
    V: SerializableValue,
{
    use_lfu: bool,
    lfu_cache: SerializableLfuCache<K, V>,
    lru_cache: SerializableLruCache<K, V>,
}

impl<K: SerializableKey, V: SerializableValue> CacheModel<K, V> 
{
    #[allow(missing_docs)]
    pub fn with(capacity: usize, use_lfu: bool) -> Self {
        let lfu_cache_capacity = if use_lfu { capacity } else { 1 };
        let lru_cache_capacity = if use_lfu { 1 } else { capacity };

        CacheModel {
            use_lfu,
            lfu_cache: SerializableLfuCache { cache: LfuCache::with_capacity(lfu_cache_capacity) },
            lru_cache: SerializableLruCache { cache: LruCache::new(NonZeroUsize::new(lru_cache_capacity).unwrap()) },
        }
    }

    #[allow(missing_docs)]
    pub fn put(&mut self, key: K, value: V) {
        if self.use_lfu {
            self.lfu_cache.cache.insert(key, value);
        } else {
            self.lru_cache.cache.put(key, value);
        }
    }

    #[allow(missing_docs)]
    pub fn get(&mut self, key: K) -> Option<&V> {
        if self.use_lfu {
            self.lfu_cache.cache.get(&key)
        } else {
            self.lru_cache.cache.get(&key)
        }
    }

    #[allow(missing_docs)]
    pub fn len(&self) -> usize {
        if self.use_lfu {
            self.lfu_cache.cache.len()
        } else {
            self.lru_cache.cache.len()
        }
    }
}

impl<K: SerializableKey, V: SerializableValue> Serialize for SerializableLfuCache<K, V> 
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

impl<K: SerializableKey, V: SerializableValue> Serialize for SerializableLruCache<K, V> 
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
struct LfuCacheVisitor<K: SerializableKey, V: SerializableValue> 
{
    marker: PhantomData<fn() -> SerializableLfuCache<K, V>>
}

impl<K: SerializableKey, V: SerializableValue> LfuCacheVisitor<K, V>
{
    fn new() -> Self {
        LfuCacheVisitor {
            marker: PhantomData
        }
    }
}

impl<'de, K: SerializableKey, V: SerializableValue> Visitor<'de> for LfuCacheVisitor<K, V>
{
    // The type that our Visitor is going to produce.
    type Value = SerializableLfuCache<K, V>;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("expects lfu cache")
    }

    // Deserialize MyMap from an abstract "map" provided by the
    // Deserializer. The MapAccess input is a callback provided by
    // the Deserializer to let us see each entry in the map.
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

impl<'de, K: SerializableKey, V: SerializableValue> Deserialize<'de> for SerializableLfuCache<K, V> 
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LfuCacheVisitor::new())
    }
}
struct LruCacheVisitor<K: SerializableKey, V: SerializableValue> 
{
    marker: PhantomData<fn() -> SerializableLruCache<K, V>>
}

impl<K: SerializableKey, V: SerializableValue> LruCacheVisitor<K, V>
{
    fn new() -> Self {
        LruCacheVisitor {
            marker: PhantomData
        }
    }
}

impl<'de, K: SerializableKey, V: SerializableValue> Visitor<'de> for LruCacheVisitor<K, V>
{
    // The type that our Visitor is going to produce.
    type Value = SerializableLruCache<K, V>;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("expects lru cache")
    }

    // Deserialize MyMap from an abstract "map" provided by the
    // Deserializer. The MapAccess input is a callback provided by
    // the Deserializer to let us see each entry in the map.
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

impl<'de, K: SerializableKey, V: SerializableValue> Deserialize<'de> for SerializableLruCache<K, V> 
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LruCacheVisitor::new())
    }
}

impl<'de, K: SerializableKey, V: SerializableValue> Deserialize<'de> for CacheModel<K, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field { UseLfu, LfuCache, LruCache }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`use_lfu` or `lfu_cache` or 'lru_cache'")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "use_lfu" => Ok(Field::UseLfu),
                            "lfu_cache" => Ok(Field::LfuCache),
                            "lru_cache" => Ok(Field::LruCache),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct DurationVisitor<K: SerializableKey, V: SerializableValue>  {
            marker: PhantomData<fn() -> CacheModel<K, V>>
        }

        impl<K: SerializableKey, V: SerializableValue> DurationVisitor<K, V>
        {
            fn new() -> Self {
                DurationVisitor {
                    marker: PhantomData
                }
            }
        }
        
        impl<'de, K: SerializableKey, V: SerializableValue> Visitor<'de> for DurationVisitor<K, V> {
            type Value = CacheModel<K, V>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct CacheModel")
            }

            fn visit_map<S>(self, mut map: S) -> Result<Self::Value, S::Error>
            where
                S: MapAccess<'de>,
            {
                let mut use_lfu:Option<bool> = None;
                let mut lfu_cache:Option<SerializableLfuCache<K, V>> = None;
                let mut lru_cache:Option<SerializableLruCache<K, V>> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::UseLfu => {
                            if use_lfu.is_some() {
                                return Err(de::Error::duplicate_field("use_lfu"));
                            }
                            use_lfu = Some(map.next_value()?);
                        }
                        Field::LfuCache => {
                            if lfu_cache.is_some() {
                                return Err(de::Error::duplicate_field("lfu_cache"));
                            }
                            lfu_cache = Some(map.next_value()?);
                        }
                        Field::LruCache => {
                            if lru_cache.is_some() {
                                return Err(de::Error::duplicate_field("lru_cache"));
                            }
                            lru_cache = Some(map.next_value()?);
                        }
                    }
                }
                let use_lfu = use_lfu.ok_or_else(|| de::Error::missing_field("lru_cache"))?;
                let lfu_cache = lfu_cache.ok_or_else(|| de::Error::missing_field("lfu_cache"))?;
                let lru_cache = lru_cache.ok_or_else(|| de::Error::missing_field("lru_cache"))?;
                Ok(CacheModel {use_lfu, lfu_cache, lru_cache})
            }
        }

        const FIELDS: &'static [&'static str] = &["use_lfu", "lfu_cache", "lru_cache"];
        deserializer.deserialize_struct("CacheModel", FIELDS, DurationVisitor::new())
    }
}