extern crate chrono;
extern crate reqwest;

use chrono::{DateTime, Duration, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct SlackMessage {
    text: String,
    ts: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SlackResponse {
    ok: bool,
    messages: Vec<SlackMessage>,
    has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct EsaErrorResponse {
    error: String,
    message: String,
}

const SLACK_TOKEN_ENV_NAME: &'static str = "SLACK_TOKEN";
const ESA_TOKEN_ENV_NAME: &'static str = "ESA_TOKEN";
const ESA_TEAMNAME_ENV_NAME: &'static str = "ESA_TEAMNAME";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let slack_token = match env::var(SLACK_TOKEN_ENV_NAME) {
        Ok(val) => val,
        Err(err) => {
            panic!("{}: {}", err, SLACK_TOKEN_ENV_NAME);
        }
    };

    let esa_token = match env::var(ESA_TOKEN_ENV_NAME) {
        Ok(val) => val,
        Err(err) => {
            panic!("{}: {}", err, ESA_TOKEN_ENV_NAME);
        }
    };

    let esa_teamname = match env::var(ESA_TEAMNAME_ENV_NAME) {
        Ok(val) => val,
        Err(err) => {
            panic!("{}: {}", err, ESA_TEAMNAME_ENV_NAME);
        }
    };

    let slack_history_url = format!(
        "https://slack.com/api/conversations.history?token={}",
        slack_token
    );
    let res = reqwest::blocking::get(&slack_history_url)?;

    if res.status() != 200 {
        panic!("Get slack history is failed");
    }

    let slack_response = res.json::<SlackResponse>();
    let post_name: String;

    match slack_response {
        Ok(res) => {
            let target_date = Local::today() - Duration::days(1);
            post_name = format!("nikki/{}", target_date.format("%Y/%m/%d"));
            let mut logs = HashMap::new();

            for message in res.messages {
                let time = message.ts.parse::<f64>()? as i64;
                let dt: DateTime<Local> = Local.timestamp(time, 0);
                if dt.date() == target_date {
                    let hour = dt.format("%H");
                    let hour_logs = logs.entry(format!("{}", hour)).or_insert(vec![]);
                    println!("{:?}", message.text);
                    hour_logs.push(message.text);
                }
            }

            if logs.len() == 0 {
                print!("No logs are detected. exit");
            } else {
                let mut post_body: String = String::from("");

                for (hour, hour_logs) in &logs {
                    post_body = format!(
                        "{} ## {}時\n\n - {}",
                        post_body,
                        hour,
                        hour_logs.join("\n - ")
                    );
                }

                let mut post_json = HashMap::new();
                post_json.insert("name", &post_name);
                post_json.insert("body_md", &post_body);
                let esa_response = reqwest::blocking::Client::new()
                    .post(&format!("https://api.esa.io/v1/teams/{}/posts", esa_teamname))
                    .header("Content-Type", "application/json")
                    .header("Authorization", format!("Bearer {}", esa_token))
                    .json(&post_json)
                    .send();
                match esa_response {
                    Ok(res) => {
                        if res.status() == 201 {
                            println!("OK");
                        } else {
                            let error_response = res.json::<EsaErrorResponse>().unwrap();
                            println!("{}: {}", error_response.error, error_response.message);
                        }
                    }
                    Err(message) => panic!(message),
                }
            }
        }
        Err(messages) => panic!(messages),
    }
    Ok(())
}
