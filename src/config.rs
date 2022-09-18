use toml;
use std::{fs, error::Error};
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize,Deserialize,Debug)]
pub struct Config {
    pub client: Clients,
    pub port: Option<u16>,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct Clients {
    pub left: Option<Client>,
    pub right: Option<Client>,
    pub top: Option<Client>,
    pub bottom: Option<Client>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Client {
    pub host_name: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u32>,
}

impl Config {
    pub fn new(path: &str) -> Result<Config, Box<dyn Error>> {
        let config = fs::read_to_string(path)?;
        let config: Config = toml::from_str::<_>(&config).unwrap();
        Ok(config)
    }
}
