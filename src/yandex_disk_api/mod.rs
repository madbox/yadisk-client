use colored::*;

use mime::Mime;
use std::str::FromStr;

use std::io;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;


mod yandex_disk_data_structures;
use yandex_disk_data_structures::*;

pub mod yandex_disk_oauth;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub yandex_disk_api_url: String,
    pub oauth_token: String,
    pub client_id: String, 
    pub client_secret: String,
    pub mounts: HashMap<String, MountPointConfig>
}

impl Config {
    pub fn new_enriched_with(config_file_name: &str, matches: &clap::ArgMatches) -> Result<Config, Box<dyn std::error::Error>> {
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
pub struct MountPointConfig {
    pub local_path: String,
    pub remote_path: String,
}


pub fn make_api_request(
    url: &str,
    conf: &crate::Config,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Making API request: {}", url.blue());
    println!("Token: {:?}", conf.oauth_token.as_str());
    let rclient = reqwest::blocking::Client::new();
    let resp = rclient
        .get(url)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("OAuth {}", conf.oauth_token.as_str()),
        )
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!("OK");
        let ct = Mime::from_str(
            resp.headers()
                .get(reqwest::header::CONTENT_TYPE)
                .unwrap()
                .to_str()?,
        )?;
        if (ct.type_() == mime::APPLICATION) && (ct.subtype() == mime::JSON) {
            Ok(resp.text()?)
        } else {
            println!("--Mime type is not application/json");
            Err("Mime type is not application/json".to_string().into())
        }
    } else {
        println!("--Responce status is not OK\n{:#?}", resp.status());
        Err("Responce status is not OK".to_string().into())
    }
}

pub fn get_info(conf: &crate::Config) -> Result<(), Box<dyn std::error::Error>> {
    let disk_object: YaDisk =
        serde_json::from_str(make_api_request(conf.yandex_disk_api_url.as_str(), &conf)?.as_str())?;

    println!("Yandex disk info:\n{:#?}", disk_object);

    Ok(())
}

pub fn get_resource_info(
    path: &str,
    conf: &crate::Config) -> Result<(), Box<dyn std::error::Error>> {

    let res: Resource =
        serde_json::from_str(make_api_request(format!("{}/resources?path={}", conf.yandex_disk_api_url, path).as_str(), &conf)?.as_str())?;

    println!("Liga:\n{:#?}", res);

    Ok(())
}

pub fn get_last(
    url: &str,
    conf: &crate::Config,
    limit: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let s: String = make_api_request(
        format!("{}/resources/last-uploaded?limit={}", url, limit).as_str(),
        &conf,
    )?;
    let rl: ResourceList = serde_json::from_str(s.as_str())?;

    println!(
        "Last content:\n{}",
        rl.items
            .iter()
            .map(|x| format!(
                " ↳ ({}) {:30} Type: {} CTime: {} MTime: {}",
                x.r#type.bright_black(),
                x.name.blue(),
                x.media_type.bright_yellow(),
                x.created.bright_black(),
                x.modified.bright_black()
            ))
            .collect::<Vec<String>>()
            .join("\n")
    );

    Ok(())
}

pub fn get_list(
    url: &str,
    conf: &crate::Config,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let s: String = make_api_request(format!("{}/resources?path={}", url, path).as_str(), &conf)?;
    let r: Resource = serde_json::from_str(s.as_str())?;

    println!(
        "Name: {}\n\
              Path: {}\n\
              File: {}\n\
              Size: {}",
        r.name, r.path, r.file, r.size
    );

    if r.r#type == "dir" {
        println!(
            "Directory content:\n{}",
            r._embedded
                .items
                .iter()
                .map(|x| format!(
                    " ↳ ({}) {:30} Type: {} CTime: {} MTime: {}",
                    x.r#type.bright_black(),
                    x.name.blue(),
                    x.media_type.bright_yellow(),
                    x.created.bright_black(),
                    x.modified.bright_black()
                ))
                .collect::<Vec<String>>()
                .join("\n")
        );
    }
    Ok(())
}

pub fn upload_file(
    url: &str,
    conf: &crate::Config,
    local_path: &str,
    remote_path: &str,
    overwrite_flag: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Attempting to upload:\nLocal:{}\nTo remote:{}",
        local_path, remote_path
    );
    let s: String = make_api_request(
        format!(
            "{}/resources/upload?path={}&overwrite={}",
            url,
            utf8_percent_encode(remote_path, NON_ALPHANUMERIC)
                .to_string()
                .as_str(),
            overwrite_flag
        )
        .as_str(),
        &conf,
    )?;
    let ui: UploadInfo = serde_json::from_str(s.as_str())?;

    println!("{:#?}", ui);

    let file = File::open(local_path)?;
    let client = reqwest::blocking::Client::new();
    let res = client.put(&ui.href).body(file).send();

    println!("{:#?}", res);

    Ok(())
}

pub fn delete_remote_file(
    url: &str,
    oauth_token: &str,
    remote_path: &str,
    permanently_flag: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Trying to delete: {}", remote_path.bright_yellow());

    let rclient = reqwest::blocking::Client::new();
    let resp = rclient
        .delete(
            format!(
                "{}/resources?path={}&force_async=false&permanently={}",
                url,
                utf8_percent_encode(remote_path, NON_ALPHANUMERIC)
                    .to_string()
                    .as_str(),
                permanently_flag
            )
            .as_str(),
        )
        .header(
            reqwest::header::AUTHORIZATION,
            format!("OAuth {}", oauth_token),
        )
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!("OK");
        Ok(())
    } else {
        println!(
            "--Response status is not OK\n{:#?}\n{:#?}",
            resp.status(),
            resp.text()
        );
        Err("Response status is not OK".to_string().into())
    }
}

pub fn download_file(
    url: &str,
    conf: &crate::Config,
    path: &str,
    target_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {

    let target = target_path.or(Path::new(path).file_name()
                                .unwrap_or_default()
                                .to_str()
                            ).unwrap();

    println!("Attempting to download:\nRemote:{}\nTo:{}", path, target);

    let s: String = make_api_request(
        format!("{}/resources/download?path={}",
                url,
                utf8_percent_encode(path, NON_ALPHANUMERIC)
                   .to_string()
                   .as_str()).as_str(),
        &conf,
    )?;
    
    let di: DownloadInfo = serde_json::from_str(s.as_str())?;

    let rclient = reqwest::blocking::Client::new();
    let mut resp = rclient
        .get(&di.href)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("OAuth {}", conf.oauth_token),
        )
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!(
            "Download request done. Data size: {}",
            resp.headers().get("content-length").unwrap().to_str()?
        );
  
        let mut out = File::create(target).expect("failed to create file");
        io::copy(&mut resp, &mut out).expect("failed to copy content");
    } else {
        println!("--Responce status is not OK\n{:#?}", resp.status());
    }

    Ok(())
}
