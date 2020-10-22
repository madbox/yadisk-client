extern crate clap;
extern crate colored;
extern crate mime;
extern crate serde_json;
#[macro_use]
extern crate text_io;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::fs;
use std::fs::metadata;
use std::fs::File;
use std::io::prelude::*;

mod cli;
mod yandex_disk_api;
use yandex_disk_api::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rusqlite::{params, Connection, Result, types::Null};
use std::thread;
use std::time::{Duration};

use md5::{Md5, Digest};

use colored::*;

use walkdir::{DirEntry, WalkDir};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    yandex_disk_api_url: String,
    oauth_token: String,
    client_id: String, 
    client_secret: String,
    mounts: HashMap<String, MountPointConfig>
}

impl Config {
    fn new_enriched_with(config_file_name: &str, matches: &clap::ArgMatches) -> Result<Config, Box<dyn std::error::Error>> {
        println!("Reading config from {}", &config_file_name.blue());

        let mut conf = Config{
            yandex_disk_api_url: "https://cloud-api.yandex.net:443/v1/disk".to_string(),
            oauth_token: "uninitialized".to_string(),
            client_id: "uninitialized".to_string(),
            client_secret: "uninitialized".to_string(),
            mounts: HashMap::new()
        };

        let file = File::open(config_file_name);
        match file {
            Ok(mut f) => {
                let mut conf_content = String::new();
                f.read_to_string(&mut conf_content)
                    .expect("Error reading value");
                conf = toml::from_str(conf_content.as_str())?;
            }
            Err(_) => println!("Error reading file"),
        }
    
        println!("Mount point count: {:#?}", conf.mounts.keys().len());
        
        // Override config parameters if any of them passed with cli
        if matches.occurrences_of("oauth_token") > 0 {
            conf.oauth_token = matches.value_of("oauth_token").unwrap().to_string()
        }
    
        // FIXME some magic number for fast check
        // Convenient conf check should be implemented
        if conf.oauth_token.len() < 5 {
            return Err(String::from("No OAuth token provided").into());
        } else {
            println!("OAuth token provided via config file or cli argument");
        }

        Ok(conf)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct MountPointConfig {
    local_path: String,
    remote_path: String,
}

fn _start_sync(){
    // 
    // make md5 for each // separate processor 

    /*  file state:
        unprocessed(discovered, no md5) -> 
        md5-processed(awaiting sync, has md5) ->
        uploaded(synced)
     */

    /* DB 
    
    create sync_state
        remote_revision
        local_revision


    create files
        path: string (cannot be null)
        filename: string (cannot be null)
        discovered_at: datetime (cannot be null)
        synced_at: datetime (null - never synced)
        processed: bool default false (cannot be null)
        md5sum: string 
    */

    /* cold-file-indexer
        drop files table
        traverse directory tree on local_mount_path,
            call discover-file for each
     */

    /* file-indexer
        open DB
        find newest(most uptodate discover_time) unprocessed file
        make md5, save to file record
        mark for awaiting_sync
     */

    /* watcher
        on file event
            fn discover-file
                find or create file record in DB (path, discover_time) with current timestamp
                __if file has changed?__
                    mark file record as unprocessed (set md5 to null)
                    set local_revision to 0 (we need to resync!)
     */

    /* sync checker

        local
            get local file list (recursive traverse)
                check sync state for each entry
        remote
            get resource list (recursive traverse)
                check sync state for each entry
                    get resource info, compare with local file
                     if local file not exists - add file record, mark for sync
                     if local file present - compare file revisions(complex)
                        if remote resource is newer, then mark file record for sync(download)
                        if remote resource is older, then mark file record for sync(upload)
     */

    /* syncer
        find newest(most uptodate discover_time) processed(has md5sum) file
        call upload_file
        set synced_ad timestamp
        save current timestamp as last_sync_datetime

        if no files to upload &
            if saved last_sync_datetime older then SYNC_INTERVAL
                get remote_revision
                if remote_revision > last_remote_revision
                    ??? get list of files to download ??? last_uploaded ???
                    download file
                     - complex. rewrite file only if it not changed since start of download
                        e.g. file is not in UNPROCESSED
                        if we should download - check md5 first. may be files already identical
                            mark file record as synced, set synced_at

        if all file records are synced - set local_revision to remote_revision

     */
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct MountMeta {
    local_revision: u128,
    remote_revision: u128,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct FileMeta {
    path: String, 
    filename: String, 
    discovered_at: i64,
    synced_at: i64,
    directory: bool,
    md5sum: String,
    // TODO add 'processing_started_at' to detect stalled or outdated processing procedures
}

fn init_mount_db(mount_name: &String) -> Result<Connection, Box<dyn std::error::Error>> {
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

fn discover_local_file(conn: &Connection, path: &str, is_dir: Option<bool>) -> Result<(), Box<dyn std::error::Error>> {
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

// This function will start deferred directory watch
fn start_mount_watch(mount_name: &String, mount: &MountPointConfig) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(format!("mount_{}.sqlite3", &mount_name))?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;
        
    println!("Registering directory watcher '{}'", mount_name);
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&mount.local_path, RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) =>  {
                println!("{}:FS event! {:#?}", &mount_name, event);
                let p = event.paths.first().unwrap().to_str().unwrap(); // Is it always first entry? Check it.
                let md = metadata(&p)?;
                discover_local_file(
                    &conn,
                    &p,
                    Some(md.is_dir()),
                )?;
                let file_or_dir_str = match md.is_dir() {
                    true => "Directory",
                    false => "File",
                };
                println!("{}: {} marked as unprocessed: {}", &mount_name, &file_or_dir_str, &p);
            },
            Err(event) => println!("watch error: {:?}", event),
        }
    };
    
    Ok(())
}

fn process_files(mount_name: &String) -> Result<(), Box<dyn std::error::Error>> {
    let ten_sec = Duration::from_secs(10);

    loop {
        thread::sleep(ten_sec);

        let conn = Connection::open(format!("mount_{}.sqlite3", mount_name))?;

        // BUG possible here. If we start multiple process_files operations simultaneously, we need some locking system.

        let mut stmt = conn.prepare("SELECT path, filename, discovered_at, synced_at, directory, md5sum FROM file_meta WHERE md5sum IS NULL AND directory = 0;")?;
        let filemeta_iter = stmt.query_map(params![], |row| {
            Ok(FileMeta {
                path: row.get(0)?,
                filename: row.get(1)?,
                discovered_at: row.get(2).unwrap_or(0),
                synced_at: row.get(3).unwrap_or(0),
                directory: row.get(4).unwrap_or(false),
                md5sum: row.get(5).unwrap_or("".to_string()),
            })
        })?;

        let mut processed_file_count = 0;
        for filemeta in filemeta_iter {
            let fm = filemeta?;
            println!("{}: Found file to process {:#?}", &mount_name, &fm.path);

            let mut hasher = Md5::new();
            hasher.update(fs::read(&fm.path).unwrap());
            let md5 = hasher.finalize();
            println!("Digest(1 reads): {:x}", &md5);

            conn.execute(
                "UPDATE file_meta SET md5sum = ?1 
                 WHERE path = ?2",
                params![format!("{:x}", &md5), fm.path],
            )?;

            processed_file_count += 1;
        }

        println!("{}: Processor cycle ended. Processed: {}", &mount_name, processed_file_count);
    }
}

fn start_all_watches(mounts: &HashMap<String, MountPointConfig>) -> Result<(), Box<dyn std::error::Error>> {
    let mut watch_handlers = vec![];
        
    //watcher threads
    for (k,v) in mounts {
        let thread_mount_name = k.clone();
        let thread_mount = v.clone();
        watch_handlers.push(thread::spawn(move || {
            println!("Registring watch '{}': at '{}'", thread_mount_name, thread_mount.local_path);
            match start_mount_watch(&thread_mount_name, &thread_mount) {
                Ok(_) => {
                    println!("Watch for {} successfully done", thread_mount_name); "Ok"
                },
                Err(err) => {
                    println!("Watch for {} was faulty. {:#?}", thread_mount_name, err); "Err"
                },
            }
        }));
        // BUG There can be an error around SQLITE_BUSY, becouse of separate sqlite connections in watcher thread and processor thread
        // processor threads
        let thread_mount_name = k.clone();
        watch_handlers.push(thread::spawn(move || {
            match process_files(&thread_mount_name) {
                Ok(_) => {
                    println!("File Processor for {} successfully done", thread_mount_name); "Ok"
                },
                Err(e) => {
                    println!("File Processor for {} was faulty. {:#?}", thread_mount_name, e); "Err"
                },
            }
        }));

    }
    
    for child in watch_handlers {
        // Wait for the thread to finish. Returns a result.
        let _ = child.join();
    }

    Ok(())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn start_local_scan(mounts: &HashMap<String, MountPointConfig>) -> Result<(), Box<dyn std::error::Error>> {
    for (k,v) in mounts {
        println!("Starting scan '{}': {}", &k.blue(), &v.local_path.blue());

        let mut file_count = 0;

        let conn = Connection::open(format!("mount_{}.sqlite3", &k))?;
        for entry in WalkDir::new(&v.local_path).into_iter().filter_entry(|e| !is_hidden(e)).filter_map(|e| e.ok() ) {
            file_count += 1;
            let file_type_char = if entry.file_type().is_dir() {"d".blue()} else {"f".green()};
            println!("{:>5}({}): {}", file_count, file_type_char, entry.path().display());
            discover_local_file(&conn, entry.path().to_str().unwrap(), Some(entry.file_type().is_dir()))?
        }
    }
    Ok(())
}

fn print_status(mounts: &HashMap<String, MountPointConfig>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Yandex disk status");

    println!("Mounts:");
    for (k,_v) in mounts {
        println!("{}", &k);
        let conn = Connection::open(format!("mount_{}.sqlite3", &k))?;
        let mut stmt = conn.prepare("SELECT * FROM
            (SELECT count(*) FROM file_meta),
            (SELECT count(*) FROM file_meta WHERE directory = 1),
            (SELECT count(*) FROM file_meta WHERE directory = 0),
            (SELECT count(*) FROM file_meta WHERE md5sum IS NULL AND directory = 0);
        ")?;

        let mut rows = stmt.query(params![]).expect("Failed to execute status DB query");
        while let Some(row) = rows.next().expect("Failed to get first row of status query") {
            println!("
                        Total entries: {:>6}
                          Directories: {:>6}
                                Files: {:>6}
                Files without digiest: {:>6}",
                row.get_raw(0).as_i64()?,
                row.get_raw(1).as_i64()?,
                row.get_raw(2).as_i64()?,
                row.get_raw(3).as_i64()?,
            )
        };
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli::init_cli();
    let conf = Config::new_enriched_with("ydclient.toml", &matches)?;
    
    for (k,_v) in &conf.mounts {
        let _conn = init_mount_db(&k);
    }

    match matches.subcommand() {
        ("list", _) => {
            let path = utf8_percent_encode(
                matches
                    .subcommand_matches("list")
                    .unwrap()
                    .value_of("path")
                    .unwrap_or_default(),
                NON_ALPHANUMERIC,
            )
            .to_string();
            get_list(conf.yandex_disk_api_url.as_str(), &conf, &path)
        }
        ("last", _) => get_last(
            conf.yandex_disk_api_url.as_str(),
            &conf,
            matches
                .subcommand_matches("last")
                .unwrap()
                .value_of("limit")
                .unwrap()
                .to_string()
                .parse::<u64>()
                .unwrap(),
        ),
        ("info", _) => get_info(&conf),
        ("download", _) => {
            let path = matches
                .subcommand_matches("download")
                .unwrap()
                .value_of("path")
                .unwrap_or_default();
            let target_path = matches
                .subcommand_matches("download")
                .unwrap()
                .value_of("target")
                .unwrap_or_default();
            download_file(
                conf.yandex_disk_api_url.as_str(),
                &conf,
                &path,
                Some(&target_path),
            )
        }
        ("upload", _) => {
            let path = matches
                .subcommand_matches("upload")
                .unwrap()
                .value_of("path")
                .unwrap_or_default();
            let remote_path = matches
                .subcommand_matches("upload")
                .unwrap()
                .value_of("remote")
                .unwrap_or_default();
            let overwrite_str = matches
                .subcommand_matches("upload")
                .unwrap()
                .value_of("overwrite")
                .unwrap_or_default();
            let mut overwrite = false;
            if overwrite_str.eq_ignore_ascii_case("true") {
                overwrite = true;
            }
            upload_file(
                conf.yandex_disk_api_url.as_str(),
                &conf,
                &path,
                &remote_path,
                overwrite,
            )
        }
        ("delete", _) => {
            let remote_path = matches
                .subcommand_matches("delete")
                .unwrap()
                .value_of("remote")
                .unwrap_or_default();
            let permanently_flag = true;
            delete_remote_file(
                conf.yandex_disk_api_url.as_str(),
                conf.oauth_token.as_str(),
                remote_path,
                permanently_flag,
            )
        }
        ("login", _) => {
            let ti: yandex_disk_oauth::TokenInfo =
                yandex_disk_oauth::cli_auth_procedure(&conf)?;
            fs::write("config", ti.access_token)?;
            Ok(())
        }
        ("watch", _) => { start_all_watches(&conf.mounts) }
        ("scan", _) => { start_local_scan(&conf.mounts) }
        ("status", _) => { print_status(&conf.mounts) }
        _ => {
            println!("No known command given. Use help please.");
            Ok(())
        }
    }
}
