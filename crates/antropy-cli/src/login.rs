//! Browser login: OAuth authorization code with PKCE and a local callback.

use crate::config::Config;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

pub const CLIENT_ID: &str = "antropy-cli";
const WAIT: Duration = Duration::from_secs(300);

#[derive(Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
}

fn random() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn open_browser(url: &str) {
    if std::env::var_os("ANTROPY_NO_BROWSER").is_some() {
        return;
    }
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener).arg(url).spawn().ok();
}

/// Waits for the browser to come back, and returns the query it brought.
async fn callback(listener: &TcpListener) -> Result<HashMap<String, String>, String> {
    let (mut socket, _) = listener.accept().await.map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 8192];
    let mut read = 0;
    while !buffer[..read].windows(4).any(|w| w == b"\r\n\r\n") && read < buffer.len() {
        let n = socket
            .read(&mut buffer[read..])
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        read += n;
    }
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .split_whitespace()
        .nth(1)
        .ok_or("the browser sent a bad request")?;
    let url = Url::parse(&format!("http://localhost{target}")).map_err(|e| e.to_string())?;

    let body = "<!doctype html><title>Antropy</title><p>Logged in. You can close this tab.</p>";
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(reply.as_bytes()).await.ok();
    Ok(url.query_pairs().into_owned().collect())
}

pub async fn login(config: &mut Config, server: &str) -> Result<(), String> {
    let server = server.trim_end_matches('/');
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}/callback");
    let (verifier, state) = (random(), random());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let url = Url::parse_with_params(
        &format!("{server}/oauth/authorize"),
        [
            ("response_type", "code"),
            ("client_id", CLIENT_ID),
            ("redirect_uri", &redirect),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("state", &state),
        ],
    )
    .map_err(|e| e.to_string())?;

    println!("Open this address to log in:\n{url}");
    open_browser(url.as_str());

    let query = tokio::time::timeout(WAIT, callback(&listener))
        .await
        .map_err(|_| "gave up waiting for the browser".to_owned())??;
    if query.get("state") != Some(&state) {
        return Err("the login reply didn't match this login".into());
    }
    let code = query.get("code").ok_or("the login was not allowed")?;

    let res = reqwest::Client::new()
        .post(format!("{server}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("login failed: {}", res.status()));
    }
    let tokens: Tokens = res.json().await.map_err(|e| e.to_string())?;
    config.server = Some(server.to_owned());
    config.access_token = Some(tokens.access_token);
    config.refresh_token = Some(tokens.refresh_token);
    config.save()?;
    println!("Logged in.");
    Ok(())
}
