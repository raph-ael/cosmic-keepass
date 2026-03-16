use keepass::{Database, DatabaseKey};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct KpEntry {
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
}

/// Open and read all entries from a .kdbx database.
pub fn open_database(path: &str, password: &str) -> Result<Vec<KpEntry>, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Cannot open database: {e}"))?;
    let key = DatabaseKey::new().with_password(password);
    let db = Database::open(&mut file, key).map_err(|e| format!("{e}"))?;

    let mut entries = Vec::new();
    collect_entries(&db.root, &mut entries);
    Ok(entries)
}

fn collect_entries(group: &keepass::db::Group, out: &mut Vec<KpEntry>) {
    for entry in &group.entries {
        out.push(KpEntry {
            title: entry.get_title().unwrap_or("").to_string(),
            username: entry.get_username().unwrap_or("").to_string(),
            password: entry.get_password().unwrap_or("").to_string(),
            url: entry.get_url().unwrap_or("").to_string(),
            notes: entry.get("Notes").unwrap_or("").to_string(),
        });
    }
    for sub in &group.groups {
        collect_entries(sub, out);
    }
}

/// Create a new empty .kdbx4 database.
pub fn create_database(path: &str, password: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let db = Database::new(Default::default());
    let key = DatabaseKey::new().with_password(password);
    let mut file =
        File::create(path).map_err(|e| format!("Cannot create database: {e}"))?;
    db.save(&mut file, key)
        .map_err(|e| format!("Failed to save database: {e}"))?;
    Ok(())
}
