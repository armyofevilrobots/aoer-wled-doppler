use ureq;

pub fn playpause(baseurl: &str, state: bool) -> Result<(), ureq::Error> {
    let url = format!("{baseurl}/api/virtuals");
    let result: serde_json::Value = ureq::get(url.as_str()).call()?.into_json()?;
    // println!("RESULT: {:?}", result);
    if let Some(serde_json::Value::Bool(ispaused)) = result.get("paused") {
        // println!("We are paused? {}", &ispaused);
        if ispaused != &state {
            // println!("Swapping states!");
            let result: serde_json::Value = ureq::put(url.as_str()).call()?.into_json()?;
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        thread::{self, sleep},
        time::Duration,
    };

    use super::*;
    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};

    #[test]
    fn test_playpause() {
        playpause("http://localhost:8888", true);
        sleep(Duration::from_secs(3));
        playpause("http://localhost:8888", false);
    }
}
