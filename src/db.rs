use rusqlite::{Connection, params};
use crate::config::Profile;
use std::path::PathBuf;
use std::fs;

/// Full database schema. All tables use IF NOT EXISTS so this can be re-run safely
/// as a migration on every startup.
const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS profiles (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    name                 TEXT NOT NULL UNIQUE,
    system_prompt        TEXT NOT NULL DEFAULT '',
    batch_size           INTEGER NOT NULL DEFAULT 20,
    batch_timeout_ms     INTEGER NOT NULL DEFAULT 100,
    max_context_messages INTEGER NOT NULL DEFAULT 20,
    model_override       TEXT,
    temperature          REAL,
    rag_enabled          INTEGER NOT NULL DEFAULT 0,
    rag_collection       TEXT,
    tts_enabled          INTEGER NOT NULL DEFAULT 0,
    tts_voice            TEXT,
    tts_speed            REAL,
    created_at           TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Stub: conversation sessions (schema defined; not written to yet)
CREATE TABLE IF NOT EXISTS conversations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    title      TEXT,
    profile_id INTEGER REFERENCES profiles(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Stub: individual messages within a conversation
CREATE TABLE IF NOT EXISTS messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id),
    role            TEXT NOT NULL,
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Stub: RAG document store
CREATE TABLE IF NOT EXISTS rag_documents (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    collection TEXT NOT NULL,
    content    TEXT NOT NULL,
    metadata   TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Stub: FTS5 full-text index for RAG keyword search
CREATE VIRTUAL TABLE IF NOT EXISTS rag_fts
    USING fts5(content, content='rag_documents', content_rowid='id');
";

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(String),
}

pub type DbResult<T> = Result<T, DbError>;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the application database at the XDG data directory.
    pub fn open() -> DbResult<Self> {
        let db_path = Self::get_db_path()?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| DbError::Io(e.to_string()))?;
        }
        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn open_in_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> DbResult<()> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    fn get_db_path() -> DbResult<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| DbError::Io("Could not determine data directory".to_string()))?
            .join("ollama-chat-gtk4");
        Ok(data_dir.join("data.db"))
    }

    pub fn get_profiles(&self) -> DbResult<Vec<Profile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, system_prompt, batch_size, batch_timeout_ms,
                    max_context_messages, model_override, temperature,
                    rag_enabled, rag_collection, tts_enabled, tts_voice, tts_speed
             FROM profiles ORDER BY name",
        )?;

        let profiles = stmt
            .query_map([], |row| {
                Ok(Profile {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    system_prompt: row.get(2)?,
                    batch_size: row.get::<_, i64>(3)? as usize,
                    batch_timeout_ms: row.get::<_, i64>(4)? as u64,
                    max_context_messages: row.get::<_, i64>(5)? as usize,
                    model_override: row.get(6)?,
                    temperature: row.get::<_, Option<f64>>(7)?.map(|v| v as f32),
                    rag_enabled: row.get::<_, i64>(8)? != 0,
                    rag_collection: row.get(9)?,
                    tts_enabled: row.get::<_, i64>(10)? != 0,
                    tts_voice: row.get(11)?,
                    tts_speed: row.get::<_, Option<f64>>(12)?.map(|v| v as f32),
                })
            })?
            .collect::<rusqlite::Result<Vec<Profile>>>()?;

        Ok(profiles)
    }

    /// Insert or update a profile. Returns the profile's database id.
    pub fn save_profile(&self, p: &Profile) -> DbResult<i64> {
        if let Some(id) = p.id {
            self.conn.execute(
                "UPDATE profiles SET
                    name=?1, system_prompt=?2, batch_size=?3, batch_timeout_ms=?4,
                    max_context_messages=?5, model_override=?6, temperature=?7,
                    rag_enabled=?8, rag_collection=?9,
                    tts_enabled=?10, tts_voice=?11, tts_speed=?12,
                    updated_at=datetime('now')
                 WHERE id=?13",
                params![
                    p.name, p.system_prompt, p.batch_size as i64,
                    p.batch_timeout_ms as i64, p.max_context_messages as i64,
                    p.model_override, p.temperature.map(|v| v as f64),
                    p.rag_enabled as i64, p.rag_collection,
                    p.tts_enabled as i64, p.tts_voice, p.tts_speed.map(|v| v as f64),
                    id
                ],
            )?;
            Ok(id)
        } else {
            self.conn.execute(
                "INSERT INTO profiles
                    (name, system_prompt, batch_size, batch_timeout_ms,
                     max_context_messages, model_override, temperature,
                     rag_enabled, rag_collection, tts_enabled, tts_voice, tts_speed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    p.name, p.system_prompt, p.batch_size as i64,
                    p.batch_timeout_ms as i64, p.max_context_messages as i64,
                    p.model_override, p.temperature.map(|v| v as f64),
                    p.rag_enabled as i64, p.rag_collection,
                    p.tts_enabled as i64, p.tts_voice, p.tts_speed.map(|v| v as f64),
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    pub fn delete_profile(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM profiles WHERE id=?1", params![id])?;
        Ok(())
    }

    // ── Conversation history ──────────────────────────────────────────────────

    /// Create a new conversation session. Returns the new row id.
    pub fn create_conversation(&self, profile_id: Option<i64>) -> DbResult<i64> {
        self.conn.execute(
            "INSERT INTO conversations (profile_id) VALUES (?1)",
            params![profile_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Append a message to a conversation and bump `updated_at` on the parent session.
    pub fn add_message(&self, conv_id: i64, role: &str, content: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO messages (conversation_id, role, content) VALUES (?1, ?2, ?3)",
            params![conv_id, role, content],
        )?;
        self.conn.execute(
            "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
            params![conv_id],
        )?;
        Ok(())
    }

    /// Set or update the display title of a conversation.
    pub fn update_conversation_title(&self, conv_id: i64, title: &str) -> DbResult<()> {
        self.conn.execute(
            "UPDATE conversations SET title = ?1 WHERE id = ?2",
            params![title, conv_id],
        )?;
        Ok(())
    }

    /// Return all conversations ordered by most-recently-updated first.
    pub fn list_conversations(&self) -> DbResult<Vec<ConversationSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, COALESCE(title, 'New conversation'), updated_at
             FROM conversations ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ConversationSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Delete a conversation and all its messages.
    pub fn delete_conversation(&self, conv_id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![conv_id])?;
        self.conn.execute("DELETE FROM conversations WHERE id = ?1", params![conv_id])?;
        Ok(())
    }

    /// Delete every conversation and every message.
    pub fn delete_all_conversations(&self) -> DbResult<()> {
        self.conn.execute("DELETE FROM messages", [])?;
        self.conn.execute("DELETE FROM conversations", [])?;
        Ok(())
    }

    /// Return all messages for a conversation in chronological order.
    pub fn get_messages(&self, conv_id: i64) -> DbResult<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content FROM messages
             WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![conv_id], |row| {
                Ok(StoredMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

// ── Value types returned by conversation queries ──────────────────────────────

pub struct ConversationSummary {
    pub id: i64,
    pub title: String,
    pub updated_at: String,
}

pub struct StoredMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_runs_migrations() {
        Database::open_in_memory().expect("in-memory DB should open without error");
    }

    #[test]
    fn profile_insert_and_retrieve() {
        let db = Database::open_in_memory().unwrap();
        let profile = Profile {
            name: "Test Profile".to_string(),
            ..Profile::default()
        };
        let id = db.save_profile(&profile).unwrap();
        assert!(id > 0);
        let profiles = db.get_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Test Profile");
        assert_eq!(profiles[0].id, Some(id));
    }

    #[test]
    fn profile_update() {
        let db = Database::open_in_memory().unwrap();
        let mut profile = Profile {
            name: "My Profile".to_string(),
            ..Profile::default()
        };
        let id = db.save_profile(&profile).unwrap();
        profile.id = Some(id);
        profile.system_prompt = "Be concise.".to_string();
        db.save_profile(&profile).unwrap();
        let profiles = db.get_profiles().unwrap();
        assert_eq!(profiles[0].system_prompt, "Be concise.");
    }

    #[test]
    fn profile_delete() {
        let db = Database::open_in_memory().unwrap();
        let profile = Profile {
            name: "Delete Me".to_string(),
            ..Profile::default()
        };
        let id = db.save_profile(&profile).unwrap();
        db.delete_profile(id).unwrap();
        let profiles = db.get_profiles().unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn profile_temperature_roundtrips() {
        let db = Database::open_in_memory().unwrap();
        let profile = Profile {
            name: "Temp Test".to_string(),
            temperature: Some(0.42),
            ..Profile::default()
        };
        let id = db.save_profile(&profile).unwrap();
        let profiles = db.get_profiles().unwrap();
        // f32 → f64 → f32 conversion; check within tolerance
        let stored_temp = profiles[0].temperature.unwrap();
        assert!((stored_temp - 0.42_f32).abs() < 0.001, "temperature={}", stored_temp);
        assert_eq!(profiles[0].id, Some(id));
    }

    #[test]
    fn create_conversation_returns_valid_id() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_conversation(None).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn add_message_persists_and_list_retrieves() {
        let db = Database::open_in_memory().unwrap();
        let conv_id = db.create_conversation(None).unwrap();
        db.add_message(conv_id, "user", "Hello").unwrap();
        db.add_message(conv_id, "assistant", "Hi there").unwrap();

        let msgs = db.get_messages(conv_id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "Hello");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "Hi there");
    }

    #[test]
    fn list_conversations_ordered_by_updated_at() {
        let db = Database::open_in_memory().unwrap();
        let id1 = db.create_conversation(None).unwrap();
        let id2 = db.create_conversation(None).unwrap();
        // Add a message to id1, bumping its updated_at to be more recent
        db.add_message(id1, "user", "ping").unwrap();

        let list = db.list_conversations().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, id1);
        assert_eq!(list[1].id, id2);
    }

    #[test]
    fn update_conversation_title_sets_title() {
        let db = Database::open_in_memory().unwrap();
        let id = db.create_conversation(None).unwrap();
        db.update_conversation_title(id, "My Chat").unwrap();

        let list = db.list_conversations().unwrap();
        assert_eq!(list[0].title, "My Chat");
    }
}
