use colored::*;

use std::str::FromStr;
use mime::Mime;

use url::{Url};

use std::io;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use std::fs::File;

mod yandex_disk_data_structures;
use yandex_disk_data_structures::*;

pub mod yandex_disk_oauth;

pub fn make_api_request(
    url: &str,
    conf: &config::Config
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Making API request: {}", url.blue());
    println!("Token: {:?}", conf.get_str("oauth_token")?.as_str());
    let rclient = reqwest::blocking::Client::new();
    let resp = rclient.get(url)
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", conf.get_str("oauth_token")?.as_str()))
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!("OK");
        let ct = Mime::from_str(resp.headers().get(reqwest::header::CONTENT_TYPE).unwrap().to_str()?)?;
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

pub fn get_info(conf: &config::Config) -> Result<(), Box<dyn std::error::Error>>{
    let disk_object: YaDisk = serde_json::from_str(
        make_api_request(conf.get_str("url")?.as_str(), conf)?.as_str())?;

    println!("Yandex disk info:\n{:#?}", disk_object);

    Ok(())
}

pub fn get_last(url: &str, conf: &config::Config, limit: u64) -> Result<(), Box<dyn std::error::Error>>{
    let s:String = make_api_request(format!("{}/resources/last-uploaded?limit={}", url, limit).as_str(), conf)?;
    let rl:ResourceList = serde_json::from_str(s.as_str())?;

    println!("Last content:\n{}",
                 rl.items.iter()
                    .map(|x| format!(" ↳ ({}) {:30} Type: {} CTime: {} MTime: {}",
                                     x.r#type.bright_black(),
                                     x.name.blue(), 
                                     x.media_type.bright_yellow(), 
                                     x.created.bright_black(), 
                                     x.modified.bright_black()))
                    .collect::<Vec<String>>().join("\n"));
                
    Ok(())
}

pub fn get_list(url: &str, conf: &config::Config, path: &str) -> Result<(), Box<dyn std::error::Error>>{
    let s:String = make_api_request(format!("{}/resources?path={}", url, path).as_str(), conf)?;
    let r:Resource = serde_json::from_str(s.as_str())?;

    println!("Name: {}\n\
              Path: {}\n\
              File: {}\n\
              Size: {}", 
              r.name,
              r.path,
              r.file,
              r.size );

    if r.r#type == "dir" { 
        println!("Directory content:\n{}",
                 r._embedded.items.iter()
                  .map(|x| format!(" ↳ ({}) {:30} Type: {} CTime: {} MTime: {}",
                                   x.r#type.bright_black(),
                                   x.name.blue(), 
                                   x.media_type.bright_yellow(), 
                                   x.created.bright_black(), 
                                   x.modified.bright_black()))
                  .collect::<Vec<String>>().join("\n"));
    }
    Ok(())
}

pub fn upload_file(
    url: &str,
    conf: &config::Config,
    local_path: &str,
    remote_path: &str,
    overwrite_flag: bool,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Attempting to upload:\nLocal:{}\nTo remote:{}", local_path, remote_path);
    let s:String = make_api_request(
            format!(
                "{}/resources/upload?path={}&overwrite={}",
                url,
                utf8_percent_encode(remote_path, NON_ALPHANUMERIC).to_string().as_str(),
                overwrite_flag).as_str()
        , conf)?;
    let ui:UploadInfo = serde_json::from_str(s.as_str())?;

    println!("{:#?}", ui);

    let file = File::open(local_path)?;
    let client = reqwest::blocking::Client::new();
    let res = client.put(&ui.href)
                .body(file)
                .send();

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
    let resp = rclient.delete(
        format!(
            "{}/resources?path={}&force_async=false&permanently={}",
            url,
            utf8_percent_encode(remote_path, NON_ALPHANUMERIC).to_string().as_str(),
            permanently_flag).as_str())
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", oauth_token))
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!("OK");
        Ok(())
    } else {
        println!("--Response status is not OK\n{:#?}\n{:#?}", resp.status(), resp.text());
        Err("Response status is not OK".to_string().into())
    }
}

pub fn download_file(
    url: &str,
    conf: &config::Config,
    path: &str, 
    target_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Attempting to download:\nRemote:{}\nTo:{}", path, target_path.unwrap_or_default());
    let s:String = make_api_request([url, "/resources/download?path=", utf8_percent_encode(path, NON_ALPHANUMERIC).to_string().as_str()].concat().as_str(), conf)?;
    let di:DownloadInfo = serde_json::from_str(s.as_str())?;


    let rclient = reqwest::blocking::Client::new();
    let mut resp = rclient.get(&di.href)
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", conf.get_str("oauth_token")?))
        .send()?;

    if resp.status() == reqwest::StatusCode::OK {
        println!("Download request done. Data size: {}", resp.headers().get("content-length").unwrap().to_str()?);
        
        let parsed = Url::parse(&di.href)?;
        let filename = parsed.query_pairs().find(|(x,_y)| x=="filename").unwrap().1.to_string();
        let target = target_path.or(Some(filename.as_str())).unwrap();

        println!("Saving as {}", target);

        let mut out = File::create(target).expect("failed to create file");
        io::copy(&mut resp, &mut out).expect("failed to copy content");
    } else {
        println!("--Responce status is not OK\n{:#?}", resp.status());
    }

    Ok(())
}