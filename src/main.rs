extern crate clap;
use clap::{Arg, App, SubCommand};

extern crate mime;
use std::str::FromStr;
use mime::Mime;
extern crate serde_json;

const BASE_API_URL: &'static str = "https://cloud-api.yandex.net:443/v1/disk";

fn make_api_request(url: &str, oauth_token: &str) -> Result<String, Box<dyn std::error::Error>> {
    println!("Making API request: {}", url);
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
    println!("Yandex disk info:\n{:#?}", serde_json::from_str::<serde_json::Value>(
        make_api_request(url, oauth_token)?.as_str()));
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

fn get_list(url: &str, oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    println!("Root dir contents:\n{}", 
        parse_json_text_to_dir_list(make_api_request(url, oauth_token)?.as_str()).join(",\n"));
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let matches = App::new("yadisk-client")
                            .version("0.2")
                            .author("Mikhail B. <m@mdbx.ru>")
                            .about("Does some things with Yandex Disk")
                            .arg(Arg::with_name("oauth-token")
                                .short("t")
                                .long("oauth-token")
                                .value_name("OAUTH_TOKEN")
                                .help("Sets Yandex API OAuth Token https://yandex.ru/dev/oauth/doc/dg/concepts/ya-oauth-intro-docpage/")
                                .required(true)
                                .takes_value(true))
                            .arg(Arg::with_name("url")
                                .short("u")
                                .long("url")
                                .value_name("URL")
                                .help("Sets a custom Yandex Disk url")
                                .takes_value(true))
                            .subcommand(SubCommand::with_name("info"))
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
                            .subcommand(SubCommand::with_name("last")
                                .about("Get last uploaded files"))
                            .get_matches();
 
    let url = matches.value_of("url").unwrap_or(BASE_API_URL);

    // TODO read token from config file
    let oauth_token = matches.value_of("oauth-token").unwrap();
        
    match matches.subcommand() {
        ("list", _) => { get_list([url,"/resources?path=%2F"].concat().as_str(), oauth_token) },
        ("last", _) => { get_last([url,"/resources/last-uploaded?limit=5"].concat().as_str(), oauth_token) },
        ("info", _) => { get_info(url, oauth_token) },
        _ => {println!("No command given. Use help please."); Ok (())}
    }


}