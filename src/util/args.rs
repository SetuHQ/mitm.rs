use colored::*;

use clap::App;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
struct Args {
    ca: String,
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u32,
    client_key: Vec<String>,
    client_cert: Vec<String>,
    client_host: Vec<String>,
    #[serde(default = "default_log_file")]
    log_file: String,
    basic_auth_user: Vec<String>,
    basic_auth_password: Vec<String>,
    basic_auth_host: Vec<String>,
}

fn default_host() -> String {
    return "localhost".to_string();
}

fn default_port() -> u32 {
    return 8080;
}

fn default_log_file() -> String {
    return "mitm_log.json".to_string();
}

pub fn parse_args(app: App) -> Args {
    let matches = app.get_matches();

    if let Some(config_file) = matches.value_of("config") {
        // read and parse the config file
        let config_contents =
            fs::read_to_string(config_file).expect("Something went wrong reading the file");

        // parse arguments from file
        let args: Args = serde_json::from_str(&config_contents).unwrap();
        args
    } else {
        println!("Command line config file not present, proceeding to parse cmd params");

        let ca = matches.value_of("ca");
        Args {
            ca: matches.value_of("ca").unwrap_or("").to_string(),
            host: matches
                .value_of("host")
                .unwrap_or(&default_host())
                .to_string(),
            port: matches.value_of("port").unwrap_or("8080"),
            client_key: matches.value_of("client_key").unwrap_or(Vec::new()),
            client_cert: matches.value_of("client_cert").unwrap_or(Vec::new()),
            client_host: matches.value_of("client_host").unwrap_or(Vec::new()),
            log_file: matches.value_of("log_file").unwrap_or(default_log_file()),
            basic_auth_user: matches.value_of("basic_auth_user").unwrap_or(Vec::new()),
            basic_auth_password: matches
                .value_of("basic_auth_password")
                .unwrap_or(Vec::new()),
            basic_auth_host: matches.value_of("basic_auth_host").unwrap_or(Vec::new()),
        }
    }
}
