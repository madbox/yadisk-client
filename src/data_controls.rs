use serde::{Deserialize, Serialize};

use rusqlite::{params, Connection, Result, types::Null};

use std::fmt;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MountMeta {
    pub local_revision: u128,
    pub remote_revision: u128,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileMeta {
    pub path: String, 
    pub filename: String, 
    pub discovered_at: i64,
    pub synced_at: i64,
    pub directory: bool,
    pub md5sum: String,
}

pub struct MountStatus {
    pub total_entries: i64,
    pub directories: i64,
    pub files: i64,
    pub files_wo_md5: i64,
}

impl fmt::Display for MountStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "
        Total entries: {:>6}
          Directories: {:>6}
                Files: {:>6}
Files without digiest: {:>6}",
            self.total_entries,
            self.directories,
            self.files,
            self.files_wo_md5,
        )
    }
}

pub fn get_mount_db_status(mount_name: &str) -> Result<MountStatus, Box<dyn std::error::Error>> {

    let conn = Connection::open(format!("mount_{}.sqlite3", &mount_name))?;
    let mut stmt = conn.prepare("SELECT * FROM
        (SELECT count(*) FROM file_meta),
        (SELECT count(*) FROM file_meta WHERE directory = 1),
        (SELECT count(*) FROM file_meta WHERE directory = 0),
        (SELECT count(*) FROM file_meta WHERE md5sum IS NULL AND directory = 0);
    ")?;

    let mut rows = stmt.query(params![]).expect("Failed to execute status DB query");
    if let Some(row) = rows.next().expect("Failed to get first row of status query") {
        Ok(MountStatus {
            total_entries: row.get(0)?,
            directories: row.get(1)?,
            files: row.get(2)?,
            files_wo_md5: row.get(3)?,
        })
    } else {
        panic!("Can'n fetch status");
    }
}

pub fn init_mount_db(mount_name: &String) -> Result<Connection, Box<dyn std::error::Error>> {
    //FIXME check mount_name format
    let conn = Connection::open(format!("mount_{}.sqlite3", mount_name))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS mount_meta (
            id              INTEGER PRIMARY KEY,
            local_revision  INTEGER,
            remote_revision INTEGER
        );",
        params![],
    )?;
        
    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_meta (
            path            VARCHAR NOT NULL PRIMARY KEY,
            filename        VARCHAR NOT NULL,
            discovered_at   DATETIME,
            synced_at       DATETIME,
            directory       BOOLEAN,
            md5sum          VARCHAR
        );",
        params![],
    )?;
        
    match conn.execute(
        "CREATE UNIQUE INDEX idx_file_meta_unique_path 
         ON file_meta (path);",
        params![],
    ) {
        Ok(_) => println!("{}: Index on filemeta created", mount_name),
        Err(err) => println!("{}: Warning: failed to create index on file_meta. Error: {:#?}", mount_name, err),
    }

    conn.execute(
        "INSERT OR IGNORE INTO mount_meta (id, local_revision, remote_revision) VALUES (?1, ?2, ?3)",
        params![1, Null, Null],
    )?;

    Ok(conn)
}

pub fn discover_local_file(conn: &Connection, path: &str, is_dir: Option<bool>) -> Result<(), Box<dyn std::error::Error>> {
    // path column is under UNIQUE index
    let r = conn.execute(
    "REPLACE INTO file_meta (path, filename, discovered_at, synced_at, directory, md5sum)
     VALUES (?1, ?2, datetime('now'), ?3, ?4, ?5);
    ",
    params![
        path,
        "some-filename",
        // set with DB function // SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string(),
        Null,
        is_dir,
        Null,
    ]);

    match r {
        Ok(_) => {
            Ok(())
        },
        Err(e) => {Err(e.into())}
    }
}