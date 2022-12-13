use super::storage::Entry;
use crate::utils::preprocess::*;
use crate::cache::{CacheModel, CacheKey, CacheValue};

/// Example
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StoreCommand {
    id: u64,
    sql: String,
}

impl Entry for StoreCommand {

    fn encode(&mut self, cache: &mut CacheModel) {
        // split sql into template and parameters
        let (template, parameters) = split_query(&self.sql);
        let cache_key = CacheKey { val: template.clone() };
        let cache_value = CacheValue { val: template.clone() };

        if let Some(index) = cache.get_index_of(cache_key.clone()) {
            // exists in cache
            // send index and parameters
            let compressed = format!("1*|*{}*|*{}", index.to_string(), parameters);

            self.sql = compressed;
        } else {
            // send template and parameters
            let uncompressed = format!("0*|*{}*|*{}", template, parameters);

            self.sql = uncompressed;
        }

        // update cache for leader
        cache.put(cache_key, cache_value);
    }

    fn decode(&mut self, cache: &mut CacheModel) {
        //println!("Decode query {}:{}", self.id, self.sql);

        let parts: Vec<&str> = self.sql.split("*|*").collect();
        if parts.len() != 3 { 
            panic!("Unexpected query: {:?}", self.sql);
        }

        let (compressed, index_or_template, parameters) = (parts[0], parts[1].to_string(), parts[2].to_string());
        let mut template = index_or_template.clone();

        if compressed == "1" {
            // compressed messsage
            let index = index_or_template.parse::<usize>().unwrap();
            if let Some((_key, value)) = cache.get_with_index(index) {
                template = value.val.clone();
            } else { 
                let index = index;
                let id = self.id;
                let sql = self.sql.clone();
                let size = cache.len();

                panic!("Query {}:{} is out of index: {}/{:?}", id, sql, index, size);
            }
        }

        // update cache for followers
        let cache_key = CacheKey { val: template.clone() };
        let cache_value = CacheValue { val: template.clone() };
        cache.put(cache_key, cache_value);
        self.sql = merge_query(template, parameters);
    }
}