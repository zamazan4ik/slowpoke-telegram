#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ImageFileId {
    file_id: std::string::String,
}

pub struct SettingsDb {
    db: sled::Db,
}

impl SettingsDb {
    pub fn new(db_location: &std::path::Path) -> anyhow::Result<Self> {
        let db = sled::open(db_location)?;
        Ok(Self { db })
    }

    pub fn get_setting(&self, name: &str) -> anyhow::Result<String> {
        let bytes = self.db.get(name)?.ok_or(anyhow!("Setting not found"))?;

        let setting = bincode::deserialize(&bytes)?;

        Ok(setting)
    }

    pub fn add_setting(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let bytes = bincode::serialize(&value)?;
        self.db.insert(key, bytes)?;
        Ok(())
    }
}
