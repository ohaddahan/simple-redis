pub mod client;
pub use client::*;

#[cfg(test)]
mod tests {
    use crate::client::redis_async_client::RedisAsyncClient;
    use crate::client::types::{EvictionPolicy, Key, Namespace, Prefix};
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[tokio::test]
    async fn init_client() {
        let client = RedisAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        let eviction_policy = client.get_eviction_policy().await.unwrap();
        assert_eq!(eviction_policy, "maxmemory-policynoeviction");
        println!("eviction_policy => {}", eviction_policy);
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
        let client = RedisAsyncClient::new(None, Namespace("Test".to_string()))
            .await
            .unwrap();
        client
            .save_entity(&prefix, &key, &entity, None)
            .await
            .unwrap();

        let from_redis = client
            .get_entity::<TestEntity>(&prefix, &key)
            .await
            .unwrap();
        assert_eq!(entity.id, from_redis.id);
        assert_eq!(entity.date, from_redis.date);
    }
}
