use crate::client::common::RedisAsyncClientTrait;
use crate::client::types::{EvictionPolicy, Key, Namespace, Prefix};
use futures::stream::StreamExt;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, AsyncIter, ScanOptions, cmd};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;

pub struct RedisRsAsyncClient {
    pub url: String,
    pub connection: ConnectionManager,
    pub namespace: Namespace,
}

impl Clone for RedisRsAsyncClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            connection: self.connection.clone(),
            namespace: self.namespace.clone(),
        }
    }
}

impl RedisRsAsyncClient {
    fn connection(&self) -> ConnectionManager {
        self.connection.clone()
    }
}

impl RedisAsyncClientTrait<RedisRsAsyncClient> for RedisRsAsyncClient {
    async fn new(url: Option<String>, namespace: Namespace) -> anyhow::Result<Self> {
        let url = url.unwrap_or(env::var("REDIS_URL")?);
        let url = if url.ends_with("#insecure") {
            url
        } else {
            format!("{}#insecure", url)
        };
        let client = redis::Client::open(url.clone())?;
        let connection = ConnectionManager::new(client).await?;
        // let connection = client.get_multiplexed_async_connection().await?;
        Ok(Self {
            url,
            connection,
            namespace,
        })
    }

    async fn set_eviction_policy(&self, eviction_policy: EvictionPolicy) -> anyhow::Result<String> {
        let _: () = cmd("CONFIG")
            .arg("SET")
            .arg("maxmemory-policy")
            .arg(eviction_policy.to_string())
            .query_async(&mut self.connection())
            .await?;
        self.get_eviction_policy().await
    }

    async fn get_eviction_policy(&self) -> anyhow::Result<String> {
        let current_policy: Vec<String> = cmd("CONFIG")
            .arg("GET")
            .arg("maxmemory-policy")
            .query_async(&mut self.connection())
            .await?;
        Ok(current_policy.join(""))
    }

    fn key(&self, prefix: &Prefix, key: &Key) -> String {
        format!("{}:{}:{}", self.namespace.0, prefix.0, key.0)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let redis_str: Option<String> = AsyncCommands::get(&mut self.connection(), key).await?;
        Ok(redis_str)
    }

    async fn set_ex(&self, key: &str, value: &str, expiry: Option<u64>) -> anyhow::Result<()> {
        match expiry {
            Some(expiry) => {
                let _: () =
                    AsyncCommands::set_ex(&mut self.connection(), key, value, expiry).await?;
            }
            None => {
                let _: () = AsyncCommands::set(&mut self.connection(), key, value).await?;
            }
        }
        Ok(())
    }

    async fn get_all(&self) -> anyhow::Result<Vec<(String, String)>> {
        let mut output: Vec<(String, String)> = Vec::new();
        let keys: Vec<String> = AsyncCommands::keys(&mut self.connection(), "*").await?;
        for key in keys {
            if let Some(value) = self.get(&key).await? {
                output.push((key, value))
            }
        }
        Ok(output)
    }

    async fn remove(&self, keys: Vec<String>) -> anyhow::Result<()> {
        let _: () = AsyncCommands::del(&mut self.connection(), keys).await?;
        Ok(())
    }

    async fn scan<T>(
        &self,
        pattern: &str,
        chunk_size: Option<usize>,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<T>>
    where
        T: DeserializeOwned + Serialize,
    {
        let chunk_size = chunk_size.unwrap_or(100);
        let limit = limit.unwrap_or(1_000);
        let opts = ScanOptions::default().with_pattern(pattern);
        let mut con = self.connection();
        let iter: AsyncIter<Option<String>> = AsyncCommands::scan_options(&mut con, opts).await?;
        let keys: Vec<Option<String>> = iter.map(Result::unwrap_or_default).collect().await;
        let keys: Vec<String> = keys.into_iter().filter_map(|i| i).collect();
        let mut output: Vec<T> = Vec::with_capacity(keys.len());
        let mut count = 0;
        for chunk in keys.chunks(chunk_size) {
            let values: Vec<Option<String>> = AsyncCommands::mget(&mut con, chunk).await?;
            let values: Vec<String> = values.into_iter().filter_map(|i| i).collect();
            count += values.len();
            for value in values {
                match serde_json::from_str::<T>(&value) {
                    Ok(v) => output.push(v),
                    Err(_) => {}
                }
            }
            if count >= limit {
                break;
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::client::common::RedisAsyncClientTrait;
    use crate::client::redis_rs_async_client::RedisRsAsyncClient;
    use crate::client::types::{EvictionPolicy, Key, Namespace, Prefix};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;
    #[tokio::test]
    async fn init_client() {
        let client = RedisRsAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        let eviction_policy = client.get_eviction_policy().await.unwrap();
        assert_eq!(eviction_policy, "maxmemory-policynoeviction");
        let eviction_policy = client
            .set_eviction_policy(EvictionPolicy::AllKeysLFU)
            .await
            .unwrap();
        assert_eq!(eviction_policy, "maxmemory-policyallkeys-lfu");
    }

    #[tokio::test]
    async fn entity_test() {
        #[derive(Serialize, Deserialize)]
        struct TestEntity {
            pub date: DateTime<Utc>,
            pub id: Uuid,
        }

        let entity = TestEntity {
            date: Utc::now(),
            id: Uuid::new_v4(),
        };
        let prefix = Prefix("TestEntity".to_string());
        let key = Key(entity.id.to_string());
        let client = RedisRsAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        client
            .save_entity(&prefix, &key, &entity, Some(10))
            .await
            .unwrap();

        let from_redis = client
            .get_entity::<TestEntity>(&prefix, &key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entity.id, from_redis.id);
        assert_eq!(entity.date, from_redis.date);
        let _ = client
            .remove_entity::<TestEntity>(&prefix, &key)
            .await
            .unwrap();
        let from_redis = client
            .get_entity::<TestEntity>(&prefix, &key)
            .await
            .unwrap();
        assert!(from_redis.is_none());
    }

    #[tokio::test]
    async fn scan_test() {
        #[derive(Serialize, Deserialize, Debug)]
        struct TestEntity {
            pub date: DateTime<Utc>,
            pub id: Uuid,
        }
        let entity = TestEntity {
            date: Utc::now(),
            id: Uuid::new_v4(),
        };
        let prefix = Prefix("TestEntity".to_string());
        let key = Key(entity.id.to_string());
        let client = RedisRsAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        client
            .save_entity(&prefix, &key, &entity, Some(10))
            .await
            .unwrap();
        let _scan_results = client
            .scan::<TestEntity>("Test*", Some(4), None)
            .await
            .unwrap();
    }
}
