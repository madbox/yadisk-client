extern crate clap;
extern crate colored;
extern crate mime;
extern crate serde_json;
#[macro_use]
extern crate text_io;

use notify::event::EventKind::*;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    yandex_disk_api_url: String,
    oauth_token: String,
    client_id: String, 
    client_secret: String,
    mounts: HashMap<String, MountPointConfig>
}

#[derive(Serialize, Deserialize, Debug)]
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

fn start_watch(/*path: &str, */mounts: &HashMap<String, MountPointConfig>) -> Result<(), Box<dyn std::error::Error>> {
    //println!("Starting watch for: {}", path);

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    //watcher.watch(path, RecursiveMode::Recursive)?;

    for (k,v) in mounts {
        println!("Registring watch '{}': at '{}'", k, v.local_path);
        watcher.watch(&v.local_path, RecursiveMode::Recursive)?;
    }

    for res in rx {
        match res {
            Ok(event) => match event.kind {
                Create(ck) => {
                    println!("Create event. {:#?} : {:#?}", ck, event.paths);
                }
                Modify(_) => {
                    println!("Modify event! {:#?}", event.paths);
                }
                Remove(_) => {
                    println!("Remove event! {:#?}", event.paths);
                }
                _ => {
                    println!("Other event! {:#?}", event);
                }
            },
            Err(event) => println!("watch error: {:?}", event),
        }
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
            start_watch(&conf.mounts)?;
            Ok(())
        }
        _ => {
            println!("No known command given. Use help please.");
            Ok(())
        }
    }
}
