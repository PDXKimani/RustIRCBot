extern crate irc;
extern crate regex;
extern crate dynamic_reload;
extern crate ctrlc;
#[macro_use]
extern crate lazy_static;
mod plugin;

use irc::client::prelude::*;
use plugin::Plugins;
use dynamic_reload::{DynamicReload,PlatformName, Search};
use std::sync::{Arc,Mutex};
use regex::Regex;

/// Global plugin state is defined here.
/// Also define static reference Regexes for improved performance.
lazy_static! {
    static ref PLUGINS: Arc<Mutex<Plugins>> = Arc::new(Mutex::new(Plugins::new()));
    static ref RELOAD_HANDLER: Arc<Mutex<DynamicReload<'static>>> = Arc::new(Mutex::new(DynamicReload::new(Some(vec!["plugins"]), Some("plugins"), Search::Backwards)));
    static ref LOAD_REGEX: Regex = Regex::new(r"!load (.*)").unwrap();
}

fn quit() {
    PLUGINS.lock().unwrap().finalize_all();
    println!("Plugins finalized, exiting.");
    std::process::exit(0);
}

/// Main Loop
fn main() {
    // Load Configuration
    let config = Config::load("config.toml").unwrap();
    // Initialize IRC client
    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&config).unwrap();
    client.identify().unwrap();
    // Register Handler
    reactor.register_client_with_handler(client, move |client, message| {
        // Print all messages to console for debugging/monitoring
        println!("{:?}", message);
        // Bot owner plugin commands, must occur here as need access to plugin globals
        if let Some(nick) = message.source_nickname() {
            if let Command::PRIVMSG(ref chan, ref msg) = message.command {
                // Reload plugins
                if msg == "!reload" && config.is_owner(nick) {
                    println!("Triggering reload.");
                    PLUGINS.lock().unwrap().finalize_all();
                    RELOAD_HANDLER.lock().unwrap().update(Plugins::reload_callback, &mut PLUGINS.lock().unwrap());
                    PLUGINS.lock().unwrap().initialize_all(client);
                    client.send_privmsg(&chan, "Reloaded plugins successfully.").unwrap();
                }
                if msg == "!listplugins" && config.is_owner(nick) {
                    println!("Printing descriptions.");
                    PLUGINS.lock().unwrap().print_descriptions(client, &chan);
                }
                if msg == "!goodbye" && config.is_owner(nick) {
                    client.send_privmsg(&chan, "Goodbye.").unwrap();
                    quit();
                }
                // Load plugin
                if let Some(caps) = LOAD_REGEX.captures(msg) {
                    if let Some(name) = caps.get(1) {
                        let name = name.as_str();
                        match RELOAD_HANDLER.lock().unwrap().add_library(&name, PlatformName::Yes) {
                            Ok(lib) => {
                                println!("Loading plugin {}", name);
                                PLUGINS.lock().unwrap().add_plugin(&lib);
                                PLUGINS.lock().unwrap().initialize_plugin(&lib,client);
                                client.send_privmsg(&chan, &format!("Successfully loaded {}", name)).unwrap();
                            }
                            Err(_) => {
                                client.send_privmsg(&chan, &format!("Unable to load {}", name)).unwrap();
                            }
                        }
                    }
                }
            }
        }
        // Pass message on to plugins
        PLUGINS.lock().unwrap().handle_message(client, &message);
        Ok(())
    });

    // Setup exit handler
    ctrlc::set_handler( || {
        quit();
    }).unwrap();

    // Kick off IRC Client
    reactor.run().unwrap();
}
