use sqlx::Error;

pub struct SqliteDatabasePoolFactory {
    db_root_path: std::path::PathBuf,
    max_connections_per_db: u32,
    database_pools: std::collections::HashMap<i64, sqlx::SqlitePool>,
    client_pool: std::collections::HashMap<i64, std::sync::Arc<ChatDatabase>>,
}

pub struct ChatDatabase {
    database_pool: sqlx::SqlitePool,
}

impl ChatDatabase {
    pub fn new(database_pool: sqlx::SqlitePool) -> Self {
        Self { database_pool }
    }

    pub async fn check_forward_message(&self, forward_message_id: &i32) -> Result<bool, Error> {
        let result = sqlx::query(
            "SELECT message_id FROM forwarded_message WHERE message_id = ? AND timestamp >= date('now', '-1 day')",
        )
        .bind(forward_message_id)
        .fetch_optional(&self.database_pool).await?;

        Ok(result.is_some())
    }

    pub async fn add_forwarded_message(
        &self,
        forward_message_id: &i32,
    ) -> Result<sqlx::sqlite::SqliteDone, Error> {
        sqlx::query("INSERT INTO forwarded_message (message_id) VALUES(?)")
            .bind(forward_message_id)
            .execute(&self.database_pool)
            .await
    }
}

impl SqliteDatabasePoolFactory {
    pub fn new(db_root_path: std::path::PathBuf, max_connections_per_db: u32) -> Self {
        Self {
            db_root_path,
            max_connections_per_db,
            database_pools: Default::default(),
            client_pool: Default::default(),
        }
    }

    pub async fn init_new_db(
        &self,
        db: sqlx::SqlitePool,
    ) -> Result<sqlx::sqlite::SqliteDone, Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS forwarded_message (
                message_id INTEGER PRIMARY KEY NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP);",
        )
        .execute(&db)
        .await
    }

    pub async fn create(&mut self, chat_id: i64) -> anyhow::Result<std::sync::Arc<ChatDatabase>> {
        if let Some(client) = self.client_pool.get(&chat_id) {
            Ok(client.clone())
        } else {
            let new_db_path = self.db_root_path.join(format!("{}.db", chat_id));

            let connection_string = format!(
                "{}",
                new_db_path
                    .to_str()
                    .ok_or(anyhow!("Cannot convert a database path to a string"))?
            );

            log::info!("{}", connection_string);

            let connection_options = sqlx::sqlite::SqliteConnectOptions::default()
                .create_if_missing(true)
                .filename(connection_string);

            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(self.max_connections_per_db)
                .connect_with(connection_options)
                .await?;

            self.init_new_db(pool.clone()).await?;

            self.database_pools.insert(chat_id, pool.clone());

            let new_client = std::sync::Arc::new(ChatDatabase::new(pool));
            self.client_pool.insert(chat_id, new_client.clone());

            Ok(new_client)
        }
    }
}
