extern crate irc;
extern crate libloading;
mod plugin;

use irc::client::prelude::*;
use plugin::PluginManager;

const LIB_PATH: &'static str = "target/debug/libprint_plugin.so"; 

fn main() {
    let config = Config::load("config.toml").unwrap();

    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&config).unwrap();
    client.identify().unwrap();
    let mut PM = PluginManager::new(&client);
    let name = "printer".to_string();
    PM.load_plugin(LIB_PATH, &name);
    reactor.register_client_with_handler(client, move |client, message| {
        PM.handle_message(&message);
        Ok(())
    });

    reactor.run().unwrap();
}
