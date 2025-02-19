use std::path::Path;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use crate::message;

struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, sqlx::Error> {
        let opts = SqliteConnectOptions::default().filename(path);
        let pool = SqlitePool::connect_with(opts).await?;
        Ok(Self { pool })
    }
    
    async fn initialize_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
               CREATE TABLE IF NOT EXISTS messages (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   from_addr TEXT NOT NULL,
                   to_addrs TEXT NOT NULL,
                   message BINARY NOT NULL
               )
               "#,
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    async fn create_message(&self, from: &str, to: &[String], message: &[u8]) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
               INSERT INTO messages (from_addr, to_addrs, message)
               VALUES (?, ?, ?)
               "#,
        )
            .bind(from)
            .bind(to.join(","))
            .bind(message)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl message::Handler for SqliteStore {
    async fn handle_message(&self, message: message::Message) -> Result<(), Box<dyn std::error::Error>> {
        self.create_message(&message.from, &message.to, &message.data).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message;
    use tokio;
    use crate::message::Handler;

    #[tokio::test]
    async fn test_create_message() {
        // Use an in-memory database for testing
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let store = SqliteStore::open(temp_file.path()).await.unwrap();

        // Initialize the table
        store.initialize_table().await.unwrap();

        // Create a test message
        let test_message = message::Message {
            sender_domain: "example.com".to_string(),
            from: "alice@example.com".to_string(),
            to: vec!["bob@example.com".to_string()],
            data: b"Hello, Bob!".to_vec(),
        };

        // Handle the message (insert into the database)
        store.handle_message(test_message).await.unwrap();

        // Verify the message was inserted
        let row: (String, String, Vec<u8>) = sqlx::query_as(
            r#"
            SELECT from_addr, to_addrs, message FROM messages
            "#,
        )
            .fetch_one(&store.pool)
            .await
            .unwrap();

        assert_eq!(row.0, "alice@example.com");
        assert_eq!(row.1, "bob@example.com");
        assert_eq!(row.2, b"Hello, Bob!");
    }
}
