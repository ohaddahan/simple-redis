#![allow(async_fn_in_trait)]
use crate::types::{EvictionPolicy, Key, Namespace, Prefix};
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
    async fn get_entity<T>(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Serialize;

    async fn save_entity<T>(
        &self,
        prefix: &Prefix,
        key: &Key,
        value: &T,
        expiry: Option<u64>,
    ) -> anyhow::Result<()>
    where
        T: DeserializeOwned + Serialize;
    async fn remove_entity<T>(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<()>;
    async fn scan<T>(&self, pattern: &str, chunk_size: usize) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned + Serialize;
}
