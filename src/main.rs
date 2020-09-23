extern crate clap;
use clap::{Arg, App, SubCommand};

extern crate mime;
use std::str::FromStr;
use mime::Mime;

use url::{Url};

extern crate serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::io;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

extern crate colored;
use colored::*;

const BASE_API_URL: &str = "https://cloud-api.yandex.net:443/v1/disk";

mod data_structures;
use data_structures::*;

fn make_api_request(
    url: &str,
    oauth_token: &str
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Making API request: {}", url.blue());
    println!("Token: {:?}", oauth_token);
    let rclient = reqwest::blocking::Client::new();
    let resp = rclient.get(url)
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", oauth_token))
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

fn get_info(url: &str, oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    let disk_object: YaDisk = serde_json::from_str(
        make_api_request(url, oauth_token)?.as_str())?;

    println!("Yandex disk info:\n{:#?}", disk_object);

    Ok(())
}

fn get_last(url: &str, oauth_token: &str, limit: u64) -> Result<(), Box<dyn std::error::Error>>{
    let s:String = make_api_request(format!("{}/resources/last-uploaded?limit={}", url, limit).as_str(), oauth_token)?;
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

fn get_list(url: &str, oauth_token: &str, path: &str) -> Result<(), Box<dyn std::error::Error>>{
    let s:String = make_api_request(format!("{}/resources?path={}", url, path).as_str(), oauth_token)?;
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

fn upload_file(
    url: &str,
    oauth_token: &str,
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
        , oauth_token)?;
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

fn download_file(
    url: &str,
    oauth_token: &str,
    path: &str, 
    target_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Attempting to download:\nRemote:{}\nTo:{}", path, target_path.unwrap_or_default());
    let s:String = make_api_request([url, "/resources/download?path=", utf8_percent_encode(path, NON_ALPHANUMERIC).to_string().as_str()].concat().as_str(), oauth_token)?;
    let di:DownloadInfo = serde_json::from_str(s.as_str())?;


    let rclient = reqwest::blocking::Client::new();
    let mut resp = rclient.get(&di.href)
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", oauth_token))
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

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let matches = App::new("yadisk-client")
                            .version("0.0.4")
                            .author("Mikhail B. <m@mdbx.ru>")
                            .about("Does some things with Yandex Disk")
                            .arg(Arg::with_name("oauth-token")
                                .short("t")
                                .long("oauth-token")
                                .value_name("OAUTH_TOKEN")
                                .help("Sets Yandex API OAuth Token https://yandex.ru/dev/oauth/doc/dg/concepts/ya-oauth-intro-docpage/")
                                .takes_value(true))
                            .arg(Arg::with_name("url")
                                .short("u")
                                .long("url")
                                .value_name("URL")
                                .help("Sets a custom Yandex Disk url")
                                .takes_value(true))
                            .arg(Arg::with_name("proxy")
                                .short("p")
                                .long("proxy")
                                .value_name("PROXY")
                                .help("Sets a internet proxy")
                                .takes_value(true))
                            .arg(Arg::with_name("config")
                                .short("c")
                                .long("config")
                                .value_name("CONFIG")
                                .help("Get configuration from file")
                                .takes_value(true))
                            .subcommand(SubCommand::with_name("info")
                                .about("Get general information about yandex disk account"))
                            .subcommand(SubCommand::with_name("last")
                                .about("Get last uploaded file list")
                                .arg(Arg::with_name("limit")
                                    .short("l")
                                    .long("limit")
                                    .default_value("5")))
                            .subcommand(SubCommand::with_name("download")
                                .about("Download single file")
                                .arg(Arg::with_name("path")
                                    .help("File name with full path to download")
                                    .index(1))
                                .arg(Arg::with_name("target")
                                    .help("Target path file will be saved to")
                                    .index(2)))
                            .subcommand(SubCommand::with_name("upload")
                                .about("Upload single file")
                                .arg(Arg::with_name("path")
                                    .help("Local filename with full path")
                                    .index(1))
                                .arg(Arg::with_name("remote")
                                    .help("Remote path file will be saved to")
                                    .index(2))
                                .arg(Arg::with_name("overwrite")
                                    .help("Overwrite file if it already exists on remote path. true|false")
                                    .long("overwrite")
                                    .value_name("overwrite")
                                    .default_value("false")))
                            .subcommand(SubCommand::with_name("list")
                                .about("Get directory listing")
                                .arg(Arg::with_name("long")
                                    .short("l")
                                    .long("long")
                                    .help("Pring additionl information on every object from list"))
                                .arg(Arg::with_name("path")
                                    .help("Sets the base path to fetch listing of. Default is root")
                                    .default_value("/")
                                    .index(1)))
                            .subcommand(SubCommand::with_name("publish")
                                .about("Publish directory and get link to STDOUT"))
                            .subcommand(SubCommand::with_name("unpublish")
                                .about("Unpublish directory"))
                            .subcommand(SubCommand::with_name("token")
                                .about("Get OAuth token proccedure. You will get URL to Yandex OAuth page")
                                .arg(Arg::with_name("newtoken")
                                    .help("Set new OAuth token. Token will be written to config file")
                                    .index(1)))
                                .get_matches();

    let mut oauth_token = String::new();

    //
    // Load config
    //

    if let Some(c) = matches.value_of("config") {
        let file = File::open(c);
        match file {
            Ok(mut f) => {
                // Note: I have a file `config.txt` that has contents `file_value`
                f.read_to_string(&mut oauth_token).expect("Error reading value");
                trim_newline(&mut oauth_token);
            }
            Err(_) => println!("Error reading file"),
        }


    } 

    // Note: this lets us override the config file value with the
    // cli argument, if provided
    if matches.occurrences_of("oauth-token") > 0 {
        oauth_token = matches.value_of("oauth-token").unwrap().to_string();
    }

    // FIXME some magic number for fast check
    // Convenient conf check should be implemented
    if oauth_token.len() < 5 { 
        return Err(String::from("No configuration provided").into());
    }

    println!("OAuth token: {}", oauth_token);

    let url = matches.value_of("url").unwrap_or(BASE_API_URL);
        
    match matches.subcommand() {
        ("list", _) => { 
            let path = utf8_percent_encode(matches.subcommand_matches("list")
                                                  .unwrap()
                                                  .value_of("path")
                                                  .unwrap_or_default(), NON_ALPHANUMERIC
                                          ).to_string();
            get_list(url, oauth_token.as_str(), &path)
        },
        ("last", _) => { get_last(url, oauth_token.as_str(), matches.subcommand_matches("last").unwrap().value_of("limit").unwrap().to_string().parse::<u64>().unwrap()) },
        ("info", _) => { get_info(url, oauth_token.as_str()) },
        ("download", _) => {
            let path = matches.subcommand_matches("download")
                                                  .unwrap()
                                                  .value_of("path")
                                                  .unwrap_or_default();
            let target_path = matches.subcommand_matches("download")
                                                  .unwrap()
                                                  .value_of("target")
                                                  .unwrap_or_default();
            download_file(url, oauth_token.as_str(), &path, Some(&target_path))
         },
         ("upload", _) => {
            let path = matches.subcommand_matches("upload")
                                                  .unwrap()
                                                  .value_of("path")
                                                  .unwrap_or_default();
            let remote_path = matches.subcommand_matches("upload")
                                                  .unwrap()
                                                  .value_of("remote")
                                                  .unwrap_or_default();
            let overwrite_str = matches.subcommand_matches("upload")
                                                  .unwrap()
                                                  .value_of("overwrite")
                                                  .unwrap_or_default();
            let mut overwrite = false;
            if overwrite_str.eq_ignore_ascii_case("true") {
                overwrite = true;
            }
            upload_file(url, oauth_token.as_str(), &path, &remote_path, overwrite)
         },
        _ => {println!("No known command given. Use help please."); Ok (())}
    }


}