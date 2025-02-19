use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::Row;

#[async_trait]
trait MessageStore {
    async fn init(&self) -> Result<(), sqlx::Error>;
    async fn create(&self, message: String) -> Result<i64, sqlx::Error>;
    async fn read(&self, id: i64) -> Result<Option<String>, sqlx::Error>;
    async fn delete(&self, id: i64) -> Result<(), sqlx::Error>;
}

struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn open(path: &str) -> Result<Self, sqlx::Error> {
        let opts = SqliteConnectOptions::default().filename(path);
        let pool = SqlitePool::connect_with(opts).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl MessageStore for SqliteStore {
    async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
               CREATE TABLE IF NOT EXISTS messages (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   message TEXT NOT NULL
               )
               "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create(&self, message: String) -> Result<i64, sqlx::Error> {
        let id = sqlx::query(
            r#"
               INSERT INTO messages (message) VALUES (?)
               "#,
        )
        .bind(message)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    async fn read(&self, id: i64) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query(
            r#"
               SELECT id, message FROM messages WHERE id = ?
               "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get("message")))
    }

    async fn delete(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
               DELETE FROM messages WHERE id = ?
               "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite_store() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let store = SqliteStore::open(path).await.unwrap();
        store.init().await.unwrap();
        println!("Initialized database");

        let id = store.create("Hello, world!".to_string()).await.unwrap();
        let message = store.read(id).await.unwrap().unwrap();
        assert_eq!(message, "Hello, world!");

        store.delete(id).await.unwrap();
        let message = store.read(id).await.unwrap();
        assert!(message.is_none());
    }
}
