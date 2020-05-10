extern crate clap;
use clap::{Arg, App};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let matches = App::new("yadisk-client")
                            .version("1.0")
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
                            .get_matches();

    let url = matches.value_of("url").unwrap_or("https://cloud-api.yandex.net:443/v1/disk/resources?path=%2F");
    println!("Value for url: {}", url);
    let oauth_token = matches.value_of("oauth-token").unwrap();
        
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