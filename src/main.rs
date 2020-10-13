extern crate clap;
extern crate colored;
extern crate mime;
extern crate serde_json;
#[macro_use]
extern crate text_io;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::fs;
use std::fs::File;
use std::io::prelude::*;

mod cli;
mod yandex_disk_api;
use yandex_disk_api::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use rusqlite::{params, Connection, Result, types::Null};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use md5::{Md5, Digest};
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    yandex_disk_api_url: String,
    oauth_token: String,
    client_id: String, 
    client_secret: String,
    mounts: HashMap<String, MountPointConfig>
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
}

fn init_mount_db(mount_name: &String) -> Result<Connection, Box<dyn std::error::Error>> {
    //FIXME check mount_name format
    let conn = Connection::open(format!("mount_{}.sqlite3", mount_name))?;

    let r = conn.execute(
        "CREATE TABLE IF NOT EXISTS mount_meta (
            id              INTEGER PRIMARY KEY,
            local_revision  INTEGER,
            remote_revision INTEGER
        );",
        params![],
    );

    println!("create table mount_meta: {:#?}", r);
        
    let r = conn.execute(
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

    println!("create table mount_meta: {:#?}", r);
        
    match conn.execute(
        "CREATE UNIQUE INDEX idx_file_meta_unique_path 
         ON file_meta (path);",
        params![],
    ) {
        Ok(_) => println!("{}: Index on filemeta created", mount_name),
        Err(err) => println!("{}: Warning: failed to create index on file_meta. Error: {:#?}", mount_name, err),
    }

    println!("create unique index on file_meta: {:#?}", r);

    conn.execute(
        "INSERT OR IGNORE INTO mount_meta (id, local_revision, remote_revision) VALUES (?1, ?2, ?3)",
        params![1, Null, Null],
    )?;

    Ok(conn)
}

fn discover_local_file(conn: &Connection, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let r = conn.execute(
    "REPLACE INTO file_meta (path, filename, discovered_at, synced_at, directory, md5sum)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6);
    ",
    params![path, "filename", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string(), Null, Null, Null]);
        
    println!("write file_meta record result: {:#?}", r);

    Ok(())
}

// This function will start deferred directory watch
fn start_mount_watch(mount_name: &String, mount: &MountPointConfig) -> Result<(), Box<dyn std::error::Error>> {
    let conn = init_mount_db(&mount_name)?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(&mount.local_path, RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) =>  {
                println!("{}:Other event! {:#?}", &mount_name, event);
                discover_local_file(
                    &conn,
                    event.paths.first().unwrap().to_str().unwrap()
                )?;
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

        let r = conn.execute(
            "SELECT * FROM file_meta WHERE md5sum IS NULL;",
            params![],
        );
        println!("{}: Select files to process: {:#?}", mount_name, r);

        let mut stmt = conn.prepare("SELECT path, filename, discovered_at, synced_at, directory, md5sum FROM file_meta WHERE md5sum IS NULL;")?;
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

        for filemeta in filemeta_iter {
            let fm = filemeta.unwrap();
            println!("{}: Found file to process {:#?}", mount_name, fm);

            let mut hasher = Md5::new();
            hasher.update(fs::read(&fm.path).unwrap());
            let md5 = hasher.finalize();
            println!("Digest(1 reads): {:x}", &md5);

            conn.execute(
                "UPDATE file_meta SET md5sum = ?1 
                 WHERE path = ?2",
                params![format!("{:x}", &md5), fm.path],
            )?;
        }
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

        // file processor
        let thread_mount_name = k.clone();
        watch_handlers.push(thread::spawn(move || {
            match process_files(&thread_mount_name) {
                Ok(_) => {
                    println!("File Processor for {} successfully done", thread_mount_name); "Ok"
                },
                Err(_) => {
                    println!("File Processor for {} was faulty ", thread_mount_name); "Err"
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli::init_cli();

    let mut conf: Config = Config{
        yandex_disk_api_url: "https://cloud-api.yandex.net:443/v1/disk".to_string(),
        oauth_token: "uninitialized".to_string(),
        client_id: "uninitialized".to_string(),
        client_secret: "uninitialized".to_string(),
        mounts: HashMap::new()
    };
        
    let mut config_file_name = "ydclient.toml";
    if let Some(c) = matches.value_of("config") {
        config_file_name = c;
    } 

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

    println!("Real config: {:#?}", conf);
    
    // Override config parameters if any of them passed with cli
    if matches.occurrences_of("oauth_token") > 0 {
        conf.oauth_token = matches.value_of("oauth_token").unwrap().to_string()
    }

    // FIXME some magic number for fast check
    // Convenient conf check should be implemented
    if conf.oauth_token.len() < 5 {
        return Err(String::from("No OAuth token provided").into());
    }

    println!("OAuth token: {}", conf.oauth_token);
    
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
                // FIXME use '?' error handling here and everywhere in this switch block
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
        ("watch", _) => {
            println!("There will be watch!");
            start_all_watches(&conf.mounts)?;
            Ok(())
        }
        _ => {
            println!("No known command given. Use help please.");
            Ok(())
        }
    }
}
