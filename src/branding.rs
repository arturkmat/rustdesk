use base::config::keys;
use hbb_common::config;

fn compiled(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|s| !s.is_empty())
}

/// Lock this build onto the self-hosted server and hide the network-ID settings.
pub fn apply() {
    let rendezvous = compiled(option_env!("RENDEZVOUS_SERVER"));
    let relay = compiled(option_env!("RELAY_SERVER")).or(rendezvous);
    let api = compiled(option_env!("API_SERVER"));
    let key = compiled(option_env!("RS_PUB_KEY"));

    if let Some(server) = rendezvous {
        *config::PROD_RENDEZVOUS_SERVER.write().unwrap() = server.to_owned();
    }

    let mut overwrite = config::OVERWRITE_SETTINGS.write().unwrap();
    if let Some(server) = rendezvous {
        overwrite.insert(
            keys::OPTION_CUSTOM_RENDEZVOUS_SERVER.to_owned(),
            server.to_owned(),
        );
    }
    if let Some(server) = relay {
        overwrite.insert(keys::OPTION_RELAY_SERVER.to_owned(), server.to_owned());
    }
    if let Some(server) = api {
        overwrite.insert(keys::OPTION_API_SERVER.to_owned(), server.to_owned());
    }
    if let Some(server) = key {
        overwrite.insert(keys::OPTION_KEY.to_owned(), server.to_owned());
    }
    drop(overwrite);

    if locks_server() {
        config::BUILTIN_SETTINGS
            .write()
            .unwrap()
            .insert(keys::OPTION_HIDE_SERVER_SETTINGS.to_owned(), "Y".to_owned());
    }
}

/// This binary was built with the self-hosted server compiled in.
pub fn locks_server() -> bool {
    compiled(option_env!("RENDEZVOUS_SERVER")).is_some()
        || compiled(option_env!("RELAY_SERVER")).is_some()
        || compiled(option_env!("API_SERVER")).is_some()
        || compiled(option_env!("RS_PUB_KEY")).is_some()
}

/// Keep the API access token off the hbbs punch/relay path.
///
/// A non-empty token plus a licence key makes the client run Pro `secure_tcp`
/// on TCP 21116 before connecting. OSS hbbs ignores the token, and that extra
/// handshake is what fails while logged-out UDP sessions still work.
pub fn rendezvous_token(_access_token: &str) -> &str {
    ""
}

pub const UPDATE_OWNER: &str = "arturkmat";
pub const UPDATE_REPO: &str = "rustdesk";
pub const UPDATE_TAG: &str = "dark-smart";

pub fn update_release_page_url() -> String {
    format!("https://github.com/{UPDATE_OWNER}/{UPDATE_REPO}/releases/tag/{UPDATE_TAG}")
}

pub fn is_fork_update_page(url: &str) -> bool {
    url.trim_end_matches('/') == update_release_page_url()
}

pub fn fork_release_api_url() -> String {
    format!("https://api.github.com/repos/{UPDATE_OWNER}/{UPDATE_REPO}/releases/tags/{UPDATE_TAG}")
}

pub fn fork_download_url(filename: &str) -> String {
    format!(
        "https://github.com/{UPDATE_OWNER}/{UPDATE_REPO}/releases/download/{UPDATE_TAG}/{filename}"
    )
}
