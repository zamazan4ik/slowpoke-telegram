pub struct SqliteDatabasePoolFactory {
    db_root_path: std::path::PathBuf,
    max_connections_per_db: u32,
    database_pools: std::collections::HashMap<i64, sqlx::SqlitePool>,
    client_pool: std::collections::HashMap<i64, std::sync::Arc<ChatDatabase>>,
}

pub struct ChatDatabase {
    database_pool: sqlx::SqlitePool,
}

#[derive(sqlx::FromRow)]
pub struct UserId(std::string::String);

impl ChatDatabase {
    pub fn new(database_pool: sqlx::SqlitePool) -> Self {
        Self { database_pool }
    }

    pub async fn check_forward_message(
        &self,
        forward_message_id: &i32,
    ) -> anyhow::Result<bool, sqlx::Error> {
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
    ) -> anyhow::Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query("INSERT INTO forwarded_message (message_id) VALUES(?)")
            .bind(forward_message_id)
            .execute(&self.database_pool)
            .await
    }

    pub async fn check_link_message(&self, link: &str) -> anyhow::Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "SELECT message_id FROM forwarded_message WHERE link = ? AND timestamp >= date('now', '-1 day')",
        )
            .bind(link)
            .fetch_optional(&self.database_pool).await?;

        Ok(result.is_some())
    }

    pub async fn add_link_message(
        &self,
        link: &str,
    ) -> Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query("INSERT INTO link (link) VALUES(?)")
            .bind(link)
            .execute(&self.database_pool)
            .await
    }

    pub async fn add_slowpoke_info(
        &self,
        user_id: i64,
    ) -> anyhow::Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query("INSERT INTO statistic (user_id) VALUES(?)")
            .bind(user_id)
            .execute(&self.database_pool)
            .await
    }

    pub async fn get_slowpoke_info(
        &self,
        period: chrono::Duration,
    ) -> futures::stream::BoxStream<Result<UserId, sqlx::Error>> {
        let stream = sqlx::query_as::<_, UserId>(
            "SELECT user_id FROM statistic WHERE timestamp >= date('now', '-? day')",
        )
        .bind(period.num_days())
        .fetch(&self.database_pool);

        stream
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

    async fn init_new_db(
        &self,
        db: sqlx::SqlitePool,
    ) -> anyhow::Result<sqlx::sqlite::SqliteDone, sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS forwarded_message (
                message_id INTEGER PRIMARY KEY NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP);",
        )
        .execute(&db)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS link (
                link TEXT PRIMARY KEY NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP);",
        )
        .execute(&db)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS statistic (
                user_id INTEGER PRIMARY KEY NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP);",
        )
        .execute(&db)
        .await
    }

    pub async fn create(&mut self, chat_id: i64) -> anyhow::Result<std::sync::Arc<ChatDatabase>> {
        if let Some(client) = self.client_pool.get(&chat_id) {
            Ok(client.clone())
        } else {
            // Database path: ROOT/chats/chat_id/storage.db

            let new_db_path = self.db_root_path.join(format!("{}", chat_id));
            std::fs::create_dir_all(&new_db_path)?;

            let new_db_path = new_db_path.join("storage.db");

            let connection_string = format!(
                "{}",
                new_db_path
                    .to_str()
                    .ok_or(anyhow!("Cannot convert a database path to a string"))?
            );

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
