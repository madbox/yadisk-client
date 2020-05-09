extern crate clap;
use clap::{Arg, App};

extern crate mime;
use std::str::FromStr;
use mime::Mime;
extern crate serde_json;
use serde_json::Value;

fn parse_json_to_dir_list(r:reqwest::blocking::Response) -> Vec<String> {
    // TODO Parse JSON responce and put file names into this vector
    let xs = vec!["1i32".to_string(),
                      "2".to_string(),
                      "3".to_string()];
                      
    let s: String = r.text().unwrap();

    println!("Inside parse: {:#?}", s);
    // let rr: &reqwest::blocking::Response = &r;
    let j = serde_json::from_str::<Value>(&s);

    println!("JSON: {:#?}", j);

    xs
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let matches = App::new("httprequester")
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

    println!("Responce status: {:#?}", resp.status());

    let media_type_1 = match resp.headers().get(reqwest::header::CONTENT_TYPE) {
        None => {
            println!("The response does not contain a Content-Type header.");
            None
        }
        Some(content_type) => {
            let content_type = Mime::from_str(content_type.to_str()?)?;
            let media_type = match (content_type.type_(), content_type.subtype()) {
                (mime::APPLICATION, mime::JSON) => { parse_json_to_dir_list(resp); "a JSON document" },
                _ => "neither text nor image",
            };

            println!("The reponse contains {}.", media_type);
            Some(media_type)
        }
    };

    println!("The reponse contains {}.", media_type_1.unwrap());

    Ok(())
}