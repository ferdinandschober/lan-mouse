use lan_mouse::{config, event, request};

pub fn main() {
    let config = config::Config::new("./config.toml").unwrap();
    let request_server = request::Server::listen(config.port.unwrap_or(42069));
    let event_server = event::Server::new(config.port.unwrap_or(42069));
}
