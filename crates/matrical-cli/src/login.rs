//! Login, two ways at once, whichever finishes first:
//! - a browser on this machine: OAuth authorization code with PKCE and a local callback;
//! - any other device, such as a phone scanning the QR code: OAuth device login
//!   (RFC 8628), for when no browser here can reach the local callback.

use crate::config::Config;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::IsTerminal;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

pub const CLIENT_ID: &str = "matrical-cli";
const WAIT: Duration = Duration::from_secs(600);
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Deserialize)]
struct Device {
    device_code: String,
    user_code: String,
    verification_uri_complete: String,
    interval: u64,
}

#[derive(Deserialize)]
struct GrantError {
    error: String,
}

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
    if std::env::var_os("MATRICAL_NO_BROWSER").is_some() {
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

    let body = "<!doctype html><title>Matrical</title><p>Logged in. You can close this tab.</p>";
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(reply.as_bytes()).await.ok();
    Ok(url.query_pairs().into_owned().collect())
}

/// Starts a device login. `None` when the server doesn't offer one, so the
/// browser login still works against it.
async fn start_device(http: &reqwest::Client, server: &str, name: &str) -> Option<Device> {
    let res = http
        .post(format!("{server}/oauth/device"))
        .form(&[("client_id", CLIENT_ID), ("name", name)])
        .send()
        .await
        .ok()?;
    res.status().is_success().then_some(())?;
    res.json().await.ok()
}

/// Asks until the device login is allowed, denied or gone.
async fn poll_device(
    http: &reqwest::Client,
    server: &str,
    device: &Device,
    name: &str,
) -> Result<Tokens, String> {
    let mut interval = Duration::from_secs(device.interval.max(1));
    loop {
        tokio::time::sleep(interval).await;
        let res = http
            .post(format!("{server}/oauth/token"))
            .form(&[
                ("grant_type", DEVICE_GRANT),
                ("device_code", &device.device_code),
                ("name", name),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            return res.json().await.map_err(|e| e.to_string());
        }
        let error = res.json::<GrantError>().await.map(|e| e.error);
        match error.as_deref() {
            Ok("authorization_pending") => {}
            Ok("slow_down") => interval += Duration::from_secs(5),
            Ok("access_denied") => return Err("the login was denied".into()),
            Ok("expired_token") => {
                return Err("the login code ran out; run `matrical login` again".into())
            }
            Ok(other) => return Err(format!("login failed: {other}")),
            Err(e) => return Err(e.to_string()),
        }
    }
}

/// Waits for the browser to come back to this machine, then trades its code.
async fn browser_login(
    http: &reqwest::Client,
    server: &str,
    listener: &TcpListener,
    redirect: &str,
    verifier: &str,
    state: &str,
    name: &str,
) -> Result<Tokens, String> {
    let query = callback(listener).await?;
    if query.get("state").map(String::as_str) != Some(state) {
        return Err("the login reply didn't match this login".into());
    }
    let code = query.get("code").ok_or("the login was not allowed")?;
    let res = http
        .post(format!("{server}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", redirect),
            ("name", name),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("login failed: {}", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

/// Two terminal rows per line of text, each cell coloured explicitly black or
/// white, so the code scans the same on a dark terminal as on a light one.
pub fn qr(text: &str) -> Option<String> {
    use qrcode::{Color, EcLevel, QrCode};
    const QUIET: usize = 2;
    let code = QrCode::with_error_correction_level(text, EcLevel::L).ok()?;
    let width = code.width();
    let colors = code.to_colors();
    let size = width + 2 * QUIET;
    let dark = |x: usize, y: usize| {
        x >= QUIET
            && y >= QUIET
            && x < width + QUIET
            && y < width + QUIET
            && colors[(y - QUIET) * width + (x - QUIET)] == Color::Dark
    };
    let mut out = String::new();
    for y in (0..size).step_by(2) {
        for x in 0..size {
            // The upper half block takes the foreground colour, the lower half the background.
            let fg = if dark(x, y) { 30 } else { 97 };
            let bg = if dark(x, y + 1) && y + 1 < size {
                40
            } else {
                107
            };
            out.push_str(&format!("\x1b[{fg};{bg}m\u{2580}"));
        }
        out.push_str("\x1b[0m\n");
    }
    Some(out)
}

/// What this login is called in the owner's list of CLI tokens, unless they
/// pick a name: the machine's own, without the `.local` macOS adds.
pub fn machine_name() -> String {
    let named = std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
    match named {
        Some(name) if !name.is_empty() => name.trim_end_matches(".local").to_owned(),
        _ => "CLI login".to_owned(),
    }
}

pub async fn login(config: &mut Config, server: &str, name: &str) -> Result<(), String> {
    let server = server.trim_end_matches('/');
    let http = reqwest::Client::new();
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
            // Offered as the login's name on the page that allows it.
            ("name", name),
        ],
    )
    .map_err(|e| e.to_string())?;
    let device = start_device(&http, server, name).await;

    println!("Open this address to log in:\n{url}");
    if let Some(device) = &device {
        println!(
            "\nOr log in from your phone. Scan this, or open the address below and enter the code:"
        );
        if std::io::stdout().is_terminal() {
            if let Some(code) = qr(&device.verification_uri_complete) {
                print!("{code}");
            }
        }
        println!("{}", device.verification_uri_complete);
        println!("Code: {}", device.user_code);
    }
    open_browser(url.as_str());

    let browser = browser_login(&http, server, &listener, &redirect, &verifier, &state, name);
    let phone = async {
        match &device {
            Some(device) => poll_device(&http, server, device, name).await,
            None => std::future::pending().await,
        }
    };
    let tokens = tokio::time::timeout(WAIT, async {
        tokio::select! {
            tokens = browser => tokens,
            tokens = phone => tokens,
        }
    })
    .await
    .map_err(|_| "gave up waiting for the login".to_owned())??;

    config.server = Some(server.to_owned());
    config.access_token = Some(tokens.access_token);
    config.refresh_token = Some(tokens.refresh_token);
    config.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::qr;

    #[test]
    fn a_qr_code_is_square_and_ends_every_line_in_the_terminal_default() {
        let code = qr("https://console.matrical.app/device?code=WDJB-MJHT").unwrap();
        let lines: Vec<&str> = code.lines().collect();
        let cells = lines[0].matches('\u{2580}').count();
        assert_eq!(lines.len(), cells.div_ceil(2));
        assert!(lines.iter().all(|l| l.matches('\u{2580}').count() == cells));
        assert!(lines.iter().all(|l| l.ends_with("\x1b[0m")));
    }
}
