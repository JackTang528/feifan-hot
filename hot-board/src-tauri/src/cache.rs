use crate::model::BoardResult;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct BoardCache {
    conn: Mutex<Connection>,
}

impl BoardCache {
    pub fn open(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir).context("create app data dir")?;
        let db_path = app_data_dir.join("board_cache.db");
        let conn = Connection::open(db_path).context("open sqlite")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS board_cache (
                id TEXT PRIMARY KEY,
                payload TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<BoardResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT payload FROM board_cache WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            let mut board: BoardResult = serde_json::from_str(&payload)?;
            board.from_cache = true;
            return Ok(Some(board));
        }
        Ok(None)
    }

    pub fn get_all(&self, ids: &[&str]) -> Result<Vec<BoardResult>> {
        ids.iter()
            .filter_map(|id| match self.get(id) {
                Ok(Some(b)) => Some(Ok(b)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect()
    }

    pub fn put(&self, board: &BoardResult) -> Result<()> {
        let payload = serde_json::to_string(board)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO board_cache (id, payload, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
            params![board.id, payload, board.updated_at],
        )?;
        Ok(())
    }
}
