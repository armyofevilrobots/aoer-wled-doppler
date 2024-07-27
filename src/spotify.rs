use crate::types::Config;
use crate::util::*;
use base64::prelude::*;


pub fn is_playing(config: &Config)->bool{
    let cred_strings = config.spotify_config.as_ref().unwrap().clone();
    let auth_string = format!("Basic {}",
                              BASE64_STANDARD.encode(format!("{}:{}",
                                                     cred_strings.client_id,
                                                     cred_strings.client_secret)));
    println!("Auth string is {}", auth_string);
    let token_body: serde_json::Value = ureq::post("https://accounts.spotify.com/api/token")
        .set("Authorization", auth_string.as_str())
        .send_form(&[("grant_type", "client_credentials")]).unwrap()
        .into_json().unwrap();

    println!("Token body is : {}", &token_body);

    let auth_string = format!("Bearer {}", &token_body.get("access_token").unwrap());

    let body: String = ureq::get("https://api.spotify.com/v1/me/player")
        .set("Authorization", auth_string.as_str())
        .call().unwrap()
        .into_json().unwrap();
    println!("Body: {}", body);
    

    
    false
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config;
    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};

    #[test]
    fn test_spotify_playing() {
        let config = load_config();
        configure_logging(log::LevelFilter::Trace, None);
        println!("Playing? {:?}", is_playing(&config.unwrap()));
    }
}
