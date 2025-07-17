#![allow(async_fn_in_trait)]

use crate::types::{EvictionPolicy, Key, Namespace, Prefix};
use anyhow::anyhow;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait RedisAsyncClientTrait<TSelf> {
    async fn new(url: Option<String>, namespace: Namespace) -> anyhow::Result<TSelf>;
    async fn set_eviction_policy(&self, eviction_policy: EvictionPolicy) -> anyhow::Result<String>;
    async fn get_eviction_policy(&self) -> anyhow::Result<String>;
    fn key(&self, prefix: &Prefix, key: &Key) -> String;
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn set_ex(&self, key: &str, value: &str, expiry: Option<u64>) -> anyhow::Result<()>;
    async fn get_all(&self) -> anyhow::Result<Vec<(String, String)>>;
    async fn remove(&self, key: &str) -> anyhow::Result<()>;
    async fn get_entity<T>(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned + Serialize,
    {
        let redis_str: Option<String> = self.get(&self.key(prefix, key)).await?;
        match redis_str {
            Some(s) => Ok(Self::des_entity(&s)?),
            None => Ok(None),
        }
    }

    async fn save_entity<T>(
        &self,
        prefix: &Prefix,
        key: &Key,
        value: &T,
        expiry: Option<u64>,
    ) -> anyhow::Result<()>
    where
        T: DeserializeOwned + Serialize,
    {
        let value_str = Self::ser_entity(value)?;
        self.set_ex(&self.key(prefix, key), &value_str, expiry)
            .await?;
        Ok(())
    }
    async fn remove_entity<T>(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<()> {
        self.remove(&self.key(prefix, key)).await?;
        Ok(())
    }
    async fn scan<T>(
        &self,
        pattern: &str,
        chunk_size: Option<usize>,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned + Serialize;

    fn des_entity<E>(redis_str: &str) -> anyhow::Result<E>
    where
        E: DeserializeOwned + Serialize,
    {
        let redis_entity: E = serde_json::from_str(&redis_str)
            .map_err(|e| anyhow!("get_entity serde_json error: {}", e))?;
        Ok(redis_entity)
    }

    fn ser_entity<E>(entity: &E) -> anyhow::Result<String>
    where
        E: DeserializeOwned + Serialize,
    {
        let value_str = serde_json::to_string(&entity)
            .map_err(|e| anyhow!("save_entity serde_json error: {}", e))?;
        Ok(value_str)
    }
}
