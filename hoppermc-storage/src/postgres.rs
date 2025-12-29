use crate::{ChunkStorage, StorageMode};
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

pub struct PostgresStorage {
    pool: Pool,
    mode: StorageMode,
}

impl PostgresStorage {
    pub async fn new(connection_string: &str, mode: StorageMode) -> Result<Self> {
        let mut cfg = Config::new();
        cfg.url = Some(connection_string.to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)
            .context("Failed to create Postgres pool")?;

        // Ensure connections work and schema exists
        let storage = Self { pool, mode };
        storage.init_schema().await?;
        
        Ok(storage)
    }

    async fn init_schema(&self) -> Result<()> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;
        
        match self.mode {
            StorageMode::Raw => {
                client.batch_execute("
                    CREATE TABLE IF NOT EXISTS chunks_raw (
                        x INT,
                        z INT,
                        data BYTEA,
                        updated_at TIMESTAMP DEFAULT NOW(),
                        PRIMARY KEY (x, z)
                    );
                ").await.context("Failed to init raw schema")?;
            }
            _ => {
                log::warn!("Schema init for mode {:?} not yet implemented", self.mode);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ChunkStorage for PostgresStorage {
    async fn save_chunk(&self, x: i32, z: i32, data: &[u8]) -> Result<()> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;

        match self.mode {
            StorageMode::Raw => {
                // Upsert logic
                client.execute(
                    "INSERT INTO chunks_raw (x, z, data, updated_at) 
                     VALUES ($1, $2, $3, NOW())
                     ON CONFLICT (x, z) DO UPDATE SET data = $3, updated_at = NOW()",
                    &[&x, &z, &data],
                ).await.context("Failed to insert chunk raw")?;
            }
            _ => anyhow::bail!("Save not implemented for mode {:?}", self.mode),
        }

        Ok(())
    }

    async fn load_chunk(&self, x: i32, z: i32) -> Result<Option<Vec<u8>>> {
        let client = self.pool.get().await.context("Failed to get DB connection")?;
        
        match self.mode {
             StorageMode::Raw => {
                 let rows = client.query(
                     "SELECT data FROM chunks_raw WHERE x = $1 AND z = $2",
                     &[&x, &z]
                 ).await?;
                 
                 if let Some(row) = rows.first() {
                     let data: Vec<u8> = row.get(0);
                     Ok(Some(data))
                 } else {
                     Ok(None)
                 }
             },
             _ => Ok(None)
        }
    }
}
