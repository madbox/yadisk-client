extern crate clap;
use clap::{Arg, App, SubCommand};

extern crate mime;
use std::str::FromStr;
use mime::Mime;
extern crate serde_json;

fn parse_json_to_dir_list(r:reqwest::blocking::Response) -> Vec<String> {
    // FIXME refactor without unwraps

    let j: serde_json::Value = r.json().unwrap();

    let items = j["_embedded"]["items"].as_array().unwrap();
    
    items.into_iter()
        .map(|o| o.get("name")
                    .unwrap()
                    .to_string())
        .collect()
}

fn get_last(_url: &str, _oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    println!("Under construction!");
    Ok(())
}

fn get_list(url: &str, oauth_token: &str) -> Result<(), Box<dyn std::error::Error>>{
    // making the request

    let rclient = reqwest::blocking::Client::new();
    let resp = rclient.get(url)
        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", oauth_token))
        .send()?;

    match (resp.status(), resp.headers().get(reqwest::header::CONTENT_TYPE)) {
        (reqwest::StatusCode::OK, Some(content_type)) => {
            let content_type = Mime::from_str(content_type.to_str()?)?;
            match (content_type.type_(), content_type.subtype()) {
                (mime::APPLICATION, mime::JSON) => println!("Root dir contents:\n{}", parse_json_to_dir_list(resp).join(",\n")),
                _ => println!("The reponse contains {:#?}.", (content_type.type_(), content_type.subtype())),
            };

            Ok(())
        }
        (_, _) => {
            println!("The response status isn't OK or has no CONTENT-TYPE header.\nResponce STATUS:{}. Headers({:#?})", resp.status(), resp.headers());
            Err("Error while fetching directory list via API".into())
        }
    }
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

    let url = matches.value_of("url").unwrap_or("https://cloud-api.yandex.net:443/v1/disk/resources?path=%2F");
    println!("Value for url: {}", url);
    // TODO read token from config file
    let oauth_token = matches.value_of("oauth-token").unwrap();
        
    match matches.subcommand() {
        ("list", _) => { get_list(url, oauth_token) },
        ("last", _) => { get_last(url, oauth_token) },
        _ => {println!("No command given. Use help please."); Ok (())}
    }


}