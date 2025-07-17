use crate::common::RedisAsyncClientTrait;
use crate::types::{EvictionPolicy, Namespace, Prefix};
use fred::bytes_utils::Str;
use fred::prelude::Config;
use fred::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::env;
use std::time::Duration;

pub struct FredAsyncClient {
    pub url: String,
    pub connection: Pool,
    pub namespace: Namespace,
}

impl Clone for FredAsyncClient {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            connection: self.connection.clone(),
            namespace: self.namespace.clone(),
        }
    }
}

impl RedisAsyncClientTrait<FredAsyncClient> for FredAsyncClient {
    async fn new(url: Option<String>, namespace: Namespace) -> anyhow::Result<FredAsyncClient> {
        let url = url.unwrap_or(env::var("REDIS_URL")?);
        let url = if url.ends_with("#insecure") {
            url
        } else {
            format!("{}#insecure", url)
        };
        let config = Config::from_url(&url)?;
        let pool_size = env::var("REDIS_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8);
        let pool = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(pool_size)?;

        pool.init().await?;

        Ok(FredAsyncClient {
            connection: pool,
            url,
            namespace,
        })
    }

    async fn set_eviction_policy(
        &self,
        _eviction_policy: EvictionPolicy,
    ) -> anyhow::Result<String> {
        Ok("todo".to_string())
    }

    async fn get_eviction_policy(&self) -> anyhow::Result<String> {
        Ok("todo".to_string())
    }

    fn key(&self, prefix: &Prefix, key: &crate::types::Key) -> String {
        format!("{}:{}:{}", self.namespace.0, prefix.0, key.0)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let redis_str: Option<String> = self.connection.get(key).await?;
        Ok(redis_str)
    }

    async fn set_ex(&self, key: &str, value: &str, expiry: Option<u64>) -> anyhow::Result<()> {
        match expiry {
            Some(expiry) => {
                let _: () = self
                    .connection
                    .set(key, value, Some(Expiration::EX(expiry as i64)), None, false)
                    .await?;
            }
            None => {
                let _: () = self.connection.set(key, value, None, None, false).await?;
            }
        }
        Ok(())
    }

    async fn get_all(&self) -> anyhow::Result<Vec<(String, String)>> {
        // let mut output: Vec<(String, String)> = Vec::new();
        // let kesy: Vec<String> = self.connection
        todo!()
    }

    async fn remove(&self, key: &str) -> anyhow::Result<()> {
        let _: () = self.connection.del(key).await?;
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
        let mut cursor: Str = "0".into();
        // break out after 1000 records
        let max_keys = limit.unwrap_or(usize::MAX);
        let chunk_size = chunk_size.unwrap_or(100);
        let mut count = 0;
        let mut output: Vec<T> = Vec::new();

        loop {
            let (new_cursor, keys): (Str, Vec<String>) = self
                .connection
                .scan_page(cursor, pattern, Some(chunk_size as u32), None)
                .await?;
            count += keys.len();
            if keys.len() > 0 {
                let values: Vec<Option<String>> = self.connection.mget(keys).await?;
                let values: Vec<String> = values.into_iter().filter_map(|i| i).collect();
                for value in values {
                    match serde_json::from_str::<T>(&value) {
                        Ok(v) => output.push(v),
                        Err(_) => {}
                    }
                }
            }
            if count >= max_keys || new_cursor == "0" {
                break;
            } else {
                cursor = new_cursor;
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::RedisAsyncClientTrait;
    use crate::fred_async_client::FredAsyncClient;
    use crate::types::{EvictionPolicy, Key, Namespace, Prefix};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[tokio::test]
    async fn init_client() {
        let client = FredAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        let eviction_policy = client.get_eviction_policy().await.unwrap();
        assert_eq!(eviction_policy, "todo");
        let eviction_policy = client
            .set_eviction_policy(EvictionPolicy::AllKeysLFU)
            .await
            .unwrap();
        assert_eq!(eviction_policy, "todo");
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
        let client = FredAsyncClient::new(None, Namespace("Test".to_string()))
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
        let client = FredAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        client
            .save_entity(&prefix, &key, &entity, Some(10))
            .await
            .unwrap();
        let _scan_results = client
            .scan::<TestEntity>("Test*", Some(5), None)
            .await
            .unwrap();
    }
}
