use hbb_common::config::{self, keys};

const RENDEZVOUS_SERVER: &str = "remote.dark-smart.pl";
const RELAY_SERVER: &str = "remote.dark-smart.pl";
const API_SERVER: &str = "http://remote.dark-smart.pl:21114";
const RS_PUB_KEY: &str = "ePLa38kujFrGQHyjOezrqHU99eOyXjknqyc9EjLhWyM";

/// Lock this build onto the self-hosted server and hide the network-ID settings.
pub fn apply() {
    *config::PROD_RENDEZVOUS_SERVER.write().unwrap() = RENDEZVOUS_SERVER.to_owned();

    let mut overwrite = config::OVERWRITE_SETTINGS.write().unwrap();
    overwrite.insert(
        keys::OPTION_CUSTOM_RENDEZVOUS_SERVER.to_owned(),
        RENDEZVOUS_SERVER.to_owned(),
    );
    overwrite.insert(
        keys::OPTION_RELAY_SERVER.to_owned(),
        RELAY_SERVER.to_owned(),
    );
    overwrite.insert(keys::OPTION_API_SERVER.to_owned(), API_SERVER.to_owned());
    overwrite.insert(keys::OPTION_KEY.to_owned(), RS_PUB_KEY.to_owned());
    drop(overwrite);

    config::BUILTIN_SETTINGS
        .write()
        .unwrap()
        .insert(keys::OPTION_HIDE_SERVER_SETTINGS.to_owned(), "Y".to_owned());
}
