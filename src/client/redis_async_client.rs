use crate::client::types::{EvictionPolicy, Key, Namespace, Prefix};
use anyhow::anyhow;
use redis::{AsyncCommands, cmd};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;

pub struct RedisAsyncClient {
    pub url: String,
    pub connection: redis::aio::MultiplexedConnection,
    pub namespace: Namespace,
}

impl Clone for RedisAsyncClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            connection: self.connection.clone(),
            namespace: self.namespace.clone(),
        }
    }
}

impl RedisAsyncClient {
    pub async fn new(url: Option<String>, namespace: Namespace) -> anyhow::Result<Self> {
        let url = url.unwrap_or(env::var("REDIS_URL")?);
        let url = if url.ends_with("#insecure") {
            url
        } else {
            format!("{}#insecure", url)
        };
        let client = redis::Client::open(url.clone())?;
        let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            url,
            connection,
            namespace,
        })
    }

    pub async fn set_eviction_policy(
        &self,
        eviction_policy: EvictionPolicy,
    ) -> anyhow::Result<String> {
        let _: () = cmd("CONFIG")
            .arg("SET")
            .arg("maxmemory-policy")
            .arg(eviction_policy.to_string())
            .query_async(&mut self.connection())
            .await?;
        self.get_eviction_policy().await
    }

    pub async fn get_eviction_policy(&self) -> anyhow::Result<String> {
        let current_policy: Vec<String> = cmd("CONFIG")
            .arg("GET")
            .arg("maxmemory-policy")
            .query_async(&mut self.connection())
            .await?;
        Ok(current_policy.join(""))
    }

    pub fn key(&self, prefix: &Prefix, key: &Key) -> String {
        format!("{}:{}:{}", self.namespace.0, prefix.0, key.0)
    }

    pub fn connection(&self) -> redis::aio::MultiplexedConnection {
        self.connection.clone()
    }

    pub async fn get_entity<T>(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<T>
    where
        T: DeserializeOwned + Serialize,
    {
        let redis_str: Option<String> = self.connection().get(self.key(prefix, key)).await?;
        match redis_str {
            Some(string) => {
                let redis_entity: T = serde_json::from_str(&string)
                    .map_err(|e| anyhow!("get_entity serde_json error: {}", e))?;
                Ok(redis_entity)
            }
            None => Err(anyhow!("Didn't find entity")),
        }
    }

    pub async fn save_entity<T>(
        &self,
        prefix: &Prefix,
        key: &Key,
        value: &T,
        expiry: Option<u64>,
    ) -> anyhow::Result<()>
    where
        T: DeserializeOwned + Serialize,
    {
        let value_str = serde_json::to_string(&value)
            .map_err(|e| anyhow!("save_entity serde_json error: {}", e))?;
        match expiry {
            Some(expiry) => {
                let _: () = self
                    .connection()
                    .set_ex(self.key(prefix, key), value_str, expiry)
                    .await?;
            }
            None => {
                let _: () = self
                    .connection()
                    .set(self.key(prefix, key), value_str)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn remove_entity(&self, prefix: &Prefix, key: &Key) -> anyhow::Result<()> {
        let _: () = self.connection().del(self.key(prefix, key)).await?;
        Ok(())
    }
}
