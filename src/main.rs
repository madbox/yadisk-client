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


mod data_controls;
use data_controls::*;

mod cli;
mod yandex_disk_api;
use yandex_disk_api::*;

use std::collections::HashMap;

use rusqlite::{params, Connection, Result};
use std::thread;
use std::time::{Duration};

use md5::{Md5, Digest};

use colored::*;

use walkdir::{DirEntry, WalkDir};

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
    println!("Yandex disk status\nMounts:");
    for (k,_v) in mounts { println!("{}:{}", &k, get_mount_db_status(&k)?) }
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
        ("info", _) => {
            let info_matches = matches.subcommand_matches("info").unwrap();
            if info_matches.is_present("path") {
                get_resource_info(info_matches.value_of("path").unwrap_or_default(), &conf)
            } else {
                get_info(&conf)
            }
        }
        ("download", _) => {
            let dmatches = matches
            .subcommand_matches("download").unwrap();
            let path = dmatches
                .value_of("path")
                .unwrap_or_default();
            download_file(
                conf.yandex_disk_api_url.as_str(),
                &conf,
                &path,
                dmatches.value_of("target"),
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
