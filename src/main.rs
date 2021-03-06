extern crate chrono;
extern crate reqwest;

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::process;

#[derive(Debug, Serialize, Deserialize)]
struct SlackMessage {
    text: String,
    ts: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SlackResponse {
    ok: bool,
    messages: Vec<SlackMessage>,
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

    let slack_response = res.json::<SlackResponse>().unwrap();
    if !slack_response.ok {
        println!("Slack response is not ok: {:?}", slack_response.messages);
        process::exit(-1);
    }

    let target_date =
        (Utc::now().with_timezone(&FixedOffset::east(9 * 3600)) - Duration::days(1)).date();
    let post_name = format!("nikki/{}", target_date.format("%Y/%m/%d"));

    let mut logs = BTreeMap::new();
    for message in slack_response.messages {
        let time = message.ts.parse::<f64>()? as i64;
        let dt: DateTime<FixedOffset> = FixedOffset::east(9 * 3600).timestamp(time, 0);

        if dt.date() == target_date {
            let hour = dt.format("%H");
            let hour_logs = logs.entry(format!("## {}時", hour)).or_insert(String::from(""));
            *hour_logs = format!("\n- {}", message.text) + hour_logs;
        }
    }

    if logs.len() == 0 {
        print!("No logs are detected. exit");
    } else {
        let mut post_body: String = String::from("");

        for (hour, hour_logs) in &logs {
            post_body = format!(
                "{}\n\n{}\n{}",
                post_body,
                hour,
                hour_logs
            );
        }

        let mut post_json = HashMap::new();
        post_json.insert("name", &post_name);
        post_json.insert("body_md", &post_body);
        let wip = String::from("false");
        post_json.insert("wip", &wip);

        let esa_response = reqwest::blocking::Client::new()
            .post(&format!(
                "https://api.esa.io/v1/teams/{}/posts",
                esa_teamname
            ))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", esa_token))
            .json(&post_json)
            .send()
            .unwrap();

        if esa_response.status() == 201 {
            println!("OK");
        } else {
            let error_response = esa_response.json::<EsaErrorResponse>().unwrap();
            println!("{}: {}", error_response.error, error_response.message);
        }
    }

    Ok(())
}
