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

const BASE_API_URL: &str = "https://cloud-api.yandex.net:443/v1/disk";

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn start_watch(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting watch for: {}", path);

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| tx.send(res).unwrap())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive)?;

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

    let mut oauth_token = String::new();

    //
    // Load config
    //

    let mut config_file_name = "ydclient.toml";

    if let Some(c) = matches.value_of("config") {
        config_file_name = c;
        let file = File::open(c);
        match file {
            Ok(mut f) => {
                // Note: I have a file `config.txt` that has contents `file_value`
                f.read_to_string(&mut oauth_token)
                    .expect("Error reading value");
                trim_newline(&mut oauth_token);
            }
            Err(_) => println!("Error reading file"),
        }
    }

    let mut settings = config::Config::default();
    settings
        // Add in `./Settings.toml`
        .merge(config::File::with_name(config_file_name))
        .unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("YDCLIENT"))
        .unwrap();

    // Note: this lets us override the config file value with the
    // cli argument, if provided
    if matches.occurrences_of("oauth_token") > 0 {
        settings.set(
            "oauth_token",
            matches.value_of("oauth_token").unwrap().to_string(),
        )?;
    }

    // FIXME some magic number for fast check
    // Convenient conf check should be implemented
    if settings.get_str("oauth_token")?.len() < 5 {
        return Err(String::from("No configuration provided").into());
    }

    println!("OAuth token: {}", settings.get_str("oauth_token")?);

    settings.set("url", matches.value_of("url").unwrap_or(BASE_API_URL))?;

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
            get_list(settings.get_str("url")?.as_str(), &settings, &path)
        }
        ("last", _) => get_last(
            settings.get_str("url")?.as_str(),
            &settings,
            matches
                .subcommand_matches("last")
                .unwrap()
                .value_of("limit")
                .unwrap()
                .to_string()
                .parse::<u64>()
                .unwrap(),
        ),
        ("info", _) => get_info(&settings),
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
                settings.get_str("url")?.as_str(),
                &settings,
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
                settings.get_str("url")?.as_str(),
                &settings,
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
                settings.get_str("url")?.as_str(),
                oauth_token.as_str(),
                remote_path,
                permanently_flag,
            )
        }
        ("login", _) => {
            let ti: yandex_disk_oauth::TokenInfo =
                yandex_disk_oauth::cli_auth_procedure(&settings)?;
            fs::write("config", ti.access_token)?;
            Ok(())
        }
        ("watch", _) => {
            println!("There will be watch!");
            let path = matches
                .subcommand_matches("watch")
                .unwrap()
                .value_of("path")
                .unwrap_or_default();
            start_watch(path)?;
            Ok(())
        }
        _ => {
            println!("No known command given. Use help please.");
            Ok(())
        }
    }
}
