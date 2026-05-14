use rusqlite::Connection;

const MIGRATIONS: &[(&str, &str)] = &[("0001_entries", include_str!("0001_entries.sql"))];

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS __migrations (name TEXT PRIMARY KEY);")?;
    for (name, sql) in MIGRATIONS {
        let exists: bool = conn
            .query_row("SELECT 1 FROM __migrations WHERE name = ?1", [name], |_| {
                Ok(true)
            })
            .unwrap_or(false);
        if !exists {
            conn.execute_batch(sql)?;
            conn.execute("INSERT INTO __migrations (name) VALUES (?1)", [name])?;
        }
    }
    Ok(())
}
