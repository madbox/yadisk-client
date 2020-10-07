use serde::{Deserialize, Serialize};

extern crate colored;
use colored::*;

//
// TokenInfo
//
const YANDEX_OAUTH_URL: &str = "https://oauth.yandex.ru";

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenInfo {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
}

fn make_reg_user_url(conf: &config::Config) -> String {
    String::from(format!(
        "https://oauth.yandex.ru/authorize?response_type=code&client_id={}",
        conf.get_str("client_id").unwrap()
    ))
}

fn get_token(
    conf: &config::Config,
    confirmation_code: &str,
) -> Result<TokenInfo, Box<dyn std::error::Error>> {
    let rclient = reqwest::blocking::Client::new();
    let resp = rclient
        .post(format!("{}/token", YANDEX_OAUTH_URL).as_str())
        //        .header(reqwest::header::AUTHORIZATION, format!("OAuth {}", encode(format!("{}:{}", CLIENT_ID, CLIENT_SECRET))))
        .form(&[
            ("client_id", conf.get_str("client_id")?.as_str()),
            ("client_secret", conf.get_str("client_secret")?.as_str()),
            ("grant_type", "authorization_code"),
            ("code", confirmation_code),
        ])
        .send()
        .unwrap();
    // grant_type=authorization_code
    // code=confirmation_code

    if resp.status() == reqwest::StatusCode::OK {
        let content: String = resp.text().unwrap();
        println!("Response OK\n{:#?}", content);
        Ok(serde_json::from_str::<TokenInfo>(content.as_str())?)
    } else {
        println!("--Responce status is not OK\n{:#?}", resp.status());
        Err("Responce status is not OK".to_string().into())
    }
}

pub fn cli_auth_procedure(conf: &config::Config) -> Result<TokenInfo, Box<dyn std::error::Error>> {
    println!(
        "Please proceed to :{}\nThan enter authorization code here:",
        make_reg_user_url(conf).bright_yellow()
    );
    let auth_code: String = read!("{}\n");
    let t = get_token(conf, auth_code.as_str())?;
    println!("{:#?}", t);
    Ok(t)
}
