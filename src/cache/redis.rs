use std::fmt::Display;

use anyhow::Result;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use serde::{Serialize, de::DeserializeOwned};

pub trait RedisCacheRecord: Serialize + DeserializeOwned {
    fn key(&self) -> String;
    fn prefix() -> &'static str;
    fn new_key<P, K>(key_suffix: P) -> String
    where
        K: Display + Default,
        P: Into<Option<K>>,
    {
        let suffix: Option<_> = key_suffix.into();
        format!("{}{}", Self::prefix(), suffix.unwrap_or_default())
    }

    fn json(&self) -> Result<String> {
        let result = serde_json::to_string(&self)?;
        Ok(result)
    }

    fn from_redis(
        conn: &mut MultiplexedConnection,
        key: &str,
    ) -> impl Future<Output = Result<Option<Self>>> + Send {
        async move {
            let resp: Option<String> = conn.get(key).await?;
            let result = match resp {
                Some(json_str) => {
                    let record = serde_json::from_str(&json_str)?;
                    Some(record)
                }
                None => None,
            };

            Ok(result)
        }
    }

    fn list_all_keys(
        conn: &mut MultiplexedConnection,
    ) -> impl Future<Output = Result<Vec<String>>> {
        async {
            let key_prefix = format!("{}*", Self::prefix());
            let result: Vec<String> = conn.keys(&key_prefix).await?;
            Ok(result)
        }
    }

    fn mget(
        conn: &mut MultiplexedConnection,
        keys: &[&str],
    ) -> impl Future<Output = Result<Vec<Option<Self>>>> + Send {
        async move {
            let result: Vec<Option<String>> = conn.mget(keys).await?;
            Ok(vec![])
        }
    }

    fn save(&self, conn: &mut MultiplexedConnection) -> impl Future<Output = Result<()>> {
        async {
            let _: () = conn.set(self.key(), self.json()?).await?;
            Ok(())
        }
    }
    fn save_ex(
        &self,
        conn: &mut MultiplexedConnection,
        seconds: u64,
    ) -> impl Future<Output = Result<()>> {
        async move {
            let _: () = conn.set_ex(self.key(), self.json()?, seconds).await?;
            Ok(())
        }
    }

    fn remove(&self, conn: &mut MultiplexedConnection) -> impl Future<Output = Result<()>> {
        async {
            let _: () = conn.del(self.key()).await?;
            Ok(())
        }
    }
}
