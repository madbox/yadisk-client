extern crate clap;
use clap::{Arg, App, SubCommand};

extern crate mime;
use std::str::FromStr;
use std::collections::HashMap;
use mime::Mime;
extern crate serde_json;
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::prelude::*;

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS, NON_ALPHANUMERIC};

const BASE_API_URL: &'static str = "https://cloud-api.yandex.net:443/v1/disk";

//
// Disk
//

#[derive(Serialize, Deserialize, Debug)]
struct YaUser {
    country: String,
    login: String,
    display_name: String,
    uid: String
}

#[derive(Serialize, Deserialize, Debug)]
struct YaDisk {
    unlimited_autoupload_enabled: bool,
    max_file_size: u64,
    total_space: u64,
    trash_size: u64,
    is_paid: bool,
    used_space: u64,
    system_folders: HashMap<String, String>,
    user: YaUser,
    revision: u64
}

//
// Resource
//

#[derive(Serialize, Deserialize, Debug)]
struct Resource {
    #[serde(default)]
    antivirus_status: String, // (undefined, optional): <Статус проверки антивирусом>,
    #[serde(default)]
    resource_id: String, // (string, optional): <Идентификатор ресурса>,
    #[serde(default)]
    share: serde_json::Value, // (ShareInfo, optional): <Информация об общей папке>,
    #[serde(default)]
    file: String, // (string, optional): <URL для скачивания файла>,
    #[serde(default)]
    size: u64, // (integer, optional): <Размер файла>,
    #[serde(default)]
    photoslice_time: String, // (string, optional): <Дата создания фото или видео файла>,
    #[serde(default)]
    _embedded: ResourceList, // (ResourceList, optional): <Список вложенных ресурсов>,
    exif: Exif, // (Exif, optional): <Метаданные медиафайла (EXIF)>,
    #[serde(default)]
    custom_properties: serde_json::Value, // (object, optional): <Пользовательские атрибуты ресурса>,
    #[serde(default)]
    media_type: String, // (string, optional): <Определённый Диском тип файла>,
    #[serde(default)]
    preview: String, // (string, optional): <URL превью файла>,
    r#type: String, // (string): <Тип>,
    #[serde(default)]
    mime_type: String, // (string, optional): <MIME-тип файла>,
    #[serde(default)]
    revision: u64, // (integer, optional): <Ревизия Диска в которой этот ресурс был изменён последний раз>,
    #[serde(default)]
    public_url: String, // (string, optional): <Публичный URL>,
    path: String, // (string): <Путь к ресурсу>,
    #[serde(default)]
    md5: String, // (string, optional): <MD5-хэш>,
    #[serde(default)]
    public_key: String, // (string, optional): <Ключ опубликованного ресурса>,
    #[serde(default)]
    sha256: String, // (string, optional): <SHA256-хэш>,
    name: String, // (string): <Имя>,
    created: String, // (string): <Дата создания>,
    modified: String, // (string): <Дата изменения>,
    #[serde(default)]
    comment_ids: serde_json::Value // (CommentIds, optional): <Идентификаторы комментариев>
}

impl Default for ResourceList {
    fn default() -> Self {
        ResourceList {
            sort: String::from("_Uninitialized"),
            items: Vec::new(),
            limit: 0,
            offset: 0,
            path: String::from("_Uninitialized"),
            total: 0
        }
    }
}

/*
#[derive(Serialize, Deserialize, Debug)]
struct ShareInfo {
    is_root: bool, // (boolean, optional): <Признак того, что папка является корневой в группе>,
    is_owned: bool, // (boolean, optional): <Признак, что текущий пользователь является владельцем общей папки>,
    rights: String // (string): <Права доступа>
}
*/

#[derive(Serialize, Deserialize, Debug)]
struct ResourceList {
    sort: String, // (string, optional): <Поле, по которому отсортирован список>,
    items: Vec<Resource>, // (array[Resource]): <Элементы списка>,
    limit: u64, // (integer, optional): <Количество элементов на странице>,
    offset: u64, // (integer, optional): <Смещение от начала списка>,
    path: String, // (string): <Путь к ресурсу, для которого построен список>,
    total: u64, // (integer, optional): <Общее количество элементов в списке>
}

#[derive(Serialize, Deserialize, Debug)]
struct Exif {
    #[serde(default)]
    date_time: String, // (string, optional): <Дата съёмки.>
}

#[derive(Serialize, Deserialize, Debug)]
struct CommentIds {
    #[serde(default)]
    private_resource: String, // (string, optional): <Идентификатор комментариев для приватных ресурсов.>,
    #[serde(default)]
    public_resource: String // (string, optional): <Идентификатор комментариев для публичных ресурсов.>
} 


fn make_api_request(url: &str, oauth_token: &str) -> Result<String, Box<dyn std::error::Error>> {
    println!("Making API request: {}", url);
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
        println!("--Responce status is not OK");
        Err("Responce status is not OK".to_string().into())
    }
}

fn get_info(url: &str, oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    let disk_object: YaDisk = serde_json::from_str(
        make_api_request(url, oauth_token)?.as_str())?;

    println!("Yandex disk info:\n{:#?}", disk_object);

    Ok(())
}

fn get_last(url: &str, _oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    println!("Url for get_last: {}", url);
    println!("Under construction!");
    Ok(())
}

fn parse_json_text_to_dir_list(s: &str) -> Vec<String> {
    // FIXME refactor without unwraps
    let j: serde_json::Value = serde_json::from_str::<serde_json::Value>(s).unwrap();
    let items = j["_embedded"]["items"].as_array().unwrap();
    
    items.into_iter()
        .map(|o| o.get("name")
                    .unwrap()
                    .to_string())
        .collect()
}

fn parse_json_text_to_dir_list2(s: &str) -> Vec<String> {
    // FIXME refactor without unwraps
    let j: serde_json::Value = serde_json::from_str::<serde_json::Value>(s).unwrap();
    let items = j["_embedded"]["items"].as_array().unwrap();
    
    items.into_iter()
        .map(|o| o.get("name")
                    .unwrap()
                    .to_string())
        .collect()
}


fn get_list(url: &str, oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    let s:String = make_api_request(url, oauth_token)?;
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
                  .map(|x| [" ↳ (", &x.r#type, ") ", &x.name, ": ", &x.media_type].concat())
                  .collect::<Vec<String>>().join("\n"));
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

    let mut clap = App::new("yadisk-client")
                            .version("0.3")
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
                                .about("Get OAuth token"));
    // println!("ddd: {}", clap.p.opts.first().unwrap().s.long.unwrap());

    let matches = clap.get_matches();
    
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

        // Note: this lets us override the config file value with the
        // cli argument, if provided
        if matches.occurrences_of("oauth-token") > 0 {
            oauth_token = matches.value_of("oauth-token").unwrap().to_string();
        }
    } else {
        oauth_token = matches.value_of("oauth-token").unwrap().to_string();
    }

    println!("OAuth token: {}", oauth_token);

    let url = matches.value_of("url").unwrap_or(BASE_API_URL);
        
    match matches.subcommand() {
        ("list", _) => { 
            let path = utf8_percent_encode(matches.subcommand_matches("list")
                                                  .unwrap()
                                                  .value_of("path")
                                                  .unwrap(), NON_ALPHANUMERIC
                                          ).to_string();
            get_list([url,"/resources?path=", &path].concat().as_str(),
                     oauth_token.as_str())
        },
        ("last", _) => { get_last([url,"/resources/last-uploaded?limit=5"].concat().as_str(), oauth_token.as_str()) },
        ("info", _) => { get_info(url, oauth_token.as_str()) },
        _ => {println!("No known command given. Use help please."); Ok (())}
    }


}