use directories::ProjectDirs;
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SavedChannel {
    pub id: i64,
    pub twitch_id: String,
    pub name: String,
    pub display_name: String,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        let config_dir = ProjectDirs::from("", "", "twitch-tui")
            .expect("Could not determine config directory")
            .config_dir()
            .to_path_buf();
        config_dir.join("twitch-tui.db")
    }

    fn run_migrations(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS channels (
                id INTEGER PRIMARY KEY,
                twitch_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                saved_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY,
                twitch_id TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                icon_url TEXT
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn save_channel(&self, twitch_id: &str, name: &str, display_name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO channels (twitch_id, name, display_name) VALUES (?1, ?2, ?3)",
            params![twitch_id, name, display_name],
        )?;
        Ok(())
    }

    pub fn remove_channel(&self, twitch_id: &str) -> Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM channels WHERE twitch_id = ?1",
            params![twitch_id],
        )?;
        Ok(affected > 0)
    }

    pub fn get_saved_channels(&self) -> Result<Vec<SavedChannel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, twitch_id, name, display_name FROM channels ORDER BY saved_at",
        )?;
        let channels = stmt
            .query_map([], |row| {
                Ok(SavedChannel {
                    id: row.get(0)?,
                    twitch_id: row.get(1)?,
                    name: row.get(2)?,
                    display_name: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(channels)
    }

    pub fn is_channel_saved(&self, twitch_id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM channels WHERE twitch_id = ?1",
            params![twitch_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let result = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_get_channels() {
        let db = Db::open_in_memory().unwrap();
        db.save_channel("123", "shroud", "shroud").unwrap();
        db.save_channel("456", "pokimane", "Pokimane").unwrap();
        let channels = db.get_saved_channels().unwrap();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].name, "shroud");
        assert_eq!(channels[1].display_name, "Pokimane");
    }

    #[test]
    fn test_remove_channel() {
        let db = Db::open_in_memory().unwrap();
        db.save_channel("123", "shroud", "shroud").unwrap();
        assert!(db.remove_channel("123").unwrap());
        let channels = db.get_saved_channels().unwrap();
        assert!(channels.is_empty());
    }

    #[test]
    fn test_is_channel_saved() {
        let db = Db::open_in_memory().unwrap();
        assert!(!db.is_channel_saved("123").unwrap());
        db.save_channel("123", "shroud", "shroud").unwrap();
        assert!(db.is_channel_saved("123").unwrap());
    }

    #[test]
    fn test_duplicate_save_is_noop() {
        let db = Db::open_in_memory().unwrap();
        db.save_channel("123", "shroud", "shroud").unwrap();
        db.save_channel("123", "shroud", "shroud").unwrap();
        assert_eq!(db.get_saved_channels().unwrap().len(), 1);
    }

    #[test]
    fn test_settings() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.get_setting("key").unwrap().is_none());
        db.set_setting("key", "value").unwrap();
        assert_eq!(db.get_setting("key").unwrap(), Some("value".to_string()));
        db.set_setting("key", "new").unwrap();
        assert_eq!(db.get_setting("key").unwrap(), Some("new".to_string()));
    }
}
