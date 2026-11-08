use crate::config::Config;
use crate::login::Tokens;
use antropy_protocol::{ApiError, UploadLink};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use url::Url;

/// Why a request did not go through.
pub struct Refused {
    pub message: String,
    /// For a refused edit, the version the content is at now, so the caller
    /// can read it again instead of retrying blindly.
    pub version: Option<u64>,
}

impl From<String> for Refused {
    fn from(message: String) -> Self {
        Self {
            message,
            version: None,
        }
    }
}

impl From<&str> for Refused {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}

// Most commands only report the wording, so a refusal flattens to it.
impl From<Refused> for String {
    fn from(refused: Refused) -> Self {
        refused.message
    }
}

pub struct Client {
    pub config: Config,
    http: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    /// Builds a URL from path parts, escaping each one.
    pub fn url(&self, parts: &[&str]) -> Result<Url, Refused> {
        let server = self
            .config
            .server
            .as_deref()
            .ok_or("run `antropy login --server <address>` first")?;
        let mut url = Url::parse(server).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "bad server address".to_owned())?
            .pop_if_empty()
            .extend(parts.iter().flat_map(|p| p.split('/')));
        Ok(url)
    }

    async fn refresh(&mut self) -> Result<bool, Refused> {
        let Some(token) = self.config.refresh_token.clone() else {
            return Ok(false);
        };
        let res = self
            .http
            .post(self.url(&["oauth", "token"])?)
            .form(&[("grant_type", "refresh_token"), ("refresh_token", &token)])
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Ok(false);
        }
        let tokens: Tokens = res.json().await.map_err(|e| e.to_string())?;
        self.config.access_token = Some(tokens.access_token);
        self.config.refresh_token = Some(tokens.refresh_token);
        self.config.save()?;
        Ok(true)
    }

    /// Sends a request, refreshing the login once if it has expired.
    pub async fn send(
        &mut self,
        method: Method,
        url: Url,
        build: impl Fn(RequestBuilder) -> RequestBuilder,
    ) -> Result<Response, Refused> {
        for retry in [true, false] {
            let token = self.config.access_token.clone().unwrap_or_default();
            let request = self.http.request(method.clone(), url.clone());
            let res = build(request.bearer_auth(token))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if res.status() == StatusCode::UNAUTHORIZED && retry && self.refresh().await? {
                continue;
            }
            return check(res).await;
        }
        Err("run `antropy login` again".into())
    }

    /// Sends a file straight to storage with an upload link. The link is its
    /// own permission, so no sign-in goes with it.
    pub async fn put_upload(&self, link: &UploadLink, bytes: Vec<u8>) -> Result<(), Refused> {
        let res = self
            .http
            .put(&link.url)
            .header("x-amz-checksum-sha256", &link.checksum)
            .body(bytes)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "storage refused the upload of {}: {}",
                link.sha256,
                res.status()
            )
            .into())
        }
    }

    /// Sends a request and reads the answer into one of the shared types.
    pub async fn fetch<T: DeserializeOwned>(
        &mut self,
        method: Method,
        url: Url,
        build: impl Fn(RequestBuilder) -> RequestBuilder,
    ) -> Result<T, Refused> {
        let res = self.send(method, url, build).await?;
        res.json().await.map_err(|e| e.to_string().into())
    }

    pub async fn get<T: DeserializeOwned>(&mut self, parts: &[&str]) -> Result<T, Refused> {
        let url = self.url(parts)?;
        self.fetch(Method::GET, url, |r| r).await
    }
}

async fn check(res: Response) -> Result<Response, Refused> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err("run `antropy login` again".into());
    }
    match res.json::<ApiError>().await {
        Ok(body) if !body.message.is_empty() => Err(Refused {
            message: body.message,
            version: body.version,
        }),
        _ => Err(format!("the server answered {status}").into()),
    }
}
