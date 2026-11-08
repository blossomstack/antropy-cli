//! Antropy command line, for people and agents.

mod client;
mod config;
mod login;
mod media;

use antropy_protocol::{
    ContentDoc, ContentEdit, ContentItem, HistoryEntry, Listing, Me, MediaRef, NewContent,
    NewSpace, SpaceMember, SpaceSummary, UploadFile, UploadLinks, UploadRequest,
};
use clap::{Parser, Subcommand};

const DEFAULT_SERVER: &str = "https://antropy.bloomstack.app";
use client::Client;
use config::Config;
use reqwest::Method;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "antropy",
    version,
    about = "Publish content to a space, and read what is there"
)]
struct Cli {
    /// Space to use, instead of the one set with `space use`.
    #[arg(long, global = true)]
    space: Option<String>,
    /// Print the raw JSON answer.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Log in through your browser.
    Login {
        /// Address of the server. Defaults to the one you last used, or
        /// https://antropy.bloomstack.app.
        #[arg(long)]
        server: Option<String>,
        /// What to call this login in your list of CLI tokens. Defaults to
        /// this machine's name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Forget this machine's login.
    Logout,
    /// Show who you're logged in as.
    Whoami,
    /// Manage spaces.
    #[command(subcommand)]
    Space(SpaceCommand),
    /// List everything in the space, each with a one-line summary.
    List {
        /// Show only what carries this tag.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Print one piece of content.
    Read {
        /// The id that `list` shows.
        id: String,
    },
    /// Publish something new, from a file or standard input.
    ///
    /// Images the markdown shows by a path next to it, such as
    /// `![Chart](./chart.png)`, are sent with it.
    Write {
        /// What people will see in the list.
        name: String,
        /// Take the text from this file instead of standard input.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Tags, separated by commas.
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// The kind of text this is.
        #[arg(long = "type", value_parser = ["markdown", "text", "json", "html"])]
        file_type: Option<String>,
        /// An image to show for it when a space is seen as tiles. Without
        /// one, the first image the text shows is used.
        #[arg(long)]
        cover: Option<PathBuf>,
    },
    /// Change something already published.
    ///
    /// New text comes from --file, or from standard input when nothing else is
    /// being changed. If somebody changed it first, the change is refused.
    /// Images found next to the file are sent again; ones it already had and
    /// still shows are kept either way.
    Update {
        /// The id that `list` shows.
        id: String,
        /// Take the text from this file instead of standard input.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Rename it.
        #[arg(long)]
        name: Option<String>,
        /// Replace its tags, separated by commas.
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// An image to show for it when a space is seen as tiles.
        #[arg(long, conflicts_with = "no_cover")]
        cover: Option<PathBuf>,
        /// Drop its chosen cover, so the first image its text shows is used.
        #[arg(long)]
        no_cover: bool,
    },
    /// Remove one piece of content.
    Rm {
        /// The id that `list` shows.
        id: String,
        /// Go ahead without asking.
        #[arg(long)]
        yes: bool,
    },
    /// Show what has changed, newest first.
    History {
        /// Follow one piece of content, by id, instead of the whole space.
        #[arg(long)]
        content: Option<String>,
    },
    /// Print instructions for agents.
    Guide,
}

#[derive(Subcommand)]
enum SpaceCommand {
    /// List your spaces.
    List,
    /// Create a space.
    Create {
        /// A short name: lowercase letters, numbers and dashes.
        slug: String,
        /// What people see. Defaults to the short name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a space and everything in it.
    Delete {
        /// A short name: lowercase letters, numbers and dashes.
        slug: String,
    },
    /// Show who can reach the space, and what they may do.
    Members,
    /// Set the space later commands use.
    Use {
        /// A short name: lowercase letters, numbers and dashes.
        slug: String,
    },
}

/// Shapes the shared protocol does not cover yet. Each key is a single word,
/// so the camelCase the wire uses cannot change them.
fn space(chosen: &Option<String>, client: &Client) -> Result<String, String> {
    chosen
        .clone()
        .or_else(|| client.config.space.clone())
        .ok_or_else(|| {
            "no space chosen: pick one with `antropy space use <name>`, or pass --space".to_owned()
        })
}

fn pretty<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}

fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The media the text shows that sits next to it on disk: beside the file,
/// or in the current folder for standard input. Files not found are left to
/// the server, which keeps those the item already has and names any missing.
fn local_media(text: &str, file: Option<&Path>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let base = file
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let mut found = Vec::new();
    for (name, _) in media::references(text) {
        let path = base.join(&name);
        if path.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            found.push((name, bytes));
        }
    }
    Ok(found)
}

/// Uploads a cover image and hands back its hash.
async fn upload_cover(client: &mut Client, slug: &str, path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hashes = upload(client, slug, vec![bytes]).await?;
    Ok(hashes.remove(0))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Uploads files straight to storage and hands back their hashes.
async fn upload(
    client: &mut Client,
    slug: &str,
    files: Vec<Vec<u8>>,
) -> Result<Vec<String>, String> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let request = UploadRequest {
        files: files
            .iter()
            .map(|bytes| UploadFile {
                sha256: sha256(bytes),
                bytes: bytes.len() as u64,
            })
            .collect(),
    };
    let url = client.url(&["api", "spaces", slug, "uploads"])?;
    let links: UploadLinks = client
        .fetch(Method::POST, url, |r| r.json(&request))
        .await?;
    let mut hashes = Vec::with_capacity(files.len());
    for (link, bytes) in links.uploads.iter().zip(files) {
        client.put_upload(link, bytes).await?;
        hashes.push(link.sha256.clone());
    }
    Ok(hashes)
}

/// Uploads a text and the media beside it, except media whose bytes the item
/// already holds. Returns the text's hash and references to what was sent.
async fn upload_text(
    client: &mut Client,
    slug: &str,
    text: String,
    file: Option<&Path>,
    held: &[String],
) -> Result<(String, Option<Vec<MediaRef>>), String> {
    let media: Vec<_> = local_media(&text, file)?
        .into_iter()
        .filter(|(_, bytes)| !held.contains(&sha256(bytes)))
        .collect();
    let mut files = vec![text.into_bytes()];
    files.extend(media.iter().map(|(_, bytes)| bytes.clone()));
    let mut hashes = upload(client, slug, files).await?.into_iter();
    let body = hashes.next().expect("the text was uploaded");
    let refs: Vec<MediaRef> = media
        .into_iter()
        .zip(hashes)
        .map(|((name, _), body)| MediaRef { name, body })
        .collect();
    Ok((body, (!refs.is_empty()).then_some(refs)))
}

/// Reads text given on standard input, saying what it is waiting for first: a
/// command that stops with no sign of why looks broken.
fn read_stdin(waiting_for: &str) -> Result<String, String> {
    eprintln!("{waiting_for}");
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Ok(text)
}

/// Asks before removing something. Anything but yes leaves it alone.
fn agreed(name: &str) -> Result<bool, String> {
    print!("Remove {name}? Type y to go ahead: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    Ok(matches!(answer.trim(), "y" | "yes"))
}

async fn space_command(
    client: &mut Client,
    chosen: &Option<String>,
    command: SpaceCommand,
    json_out: bool,
) -> Result<(), String> {
    match command {
        SpaceCommand::List => {
            let spaces: Vec<SpaceSummary> = client.get(&["api", "spaces"]).await?;
            if json_out {
                println!("{}", pretty(&spaces)?);
            } else {
                for one in &spaces {
                    println!("{}  {}  ({})", one.slug, one.name, one.role);
                }
            }
        }
        SpaceCommand::Create { slug, name } => {
            let body = NewSpace {
                slug: slug.clone(),
                name,
            };
            let url = client.url(&["api", "spaces"])?;
            client.send(Method::POST, url, |r| r.json(&body)).await?;
            println!("Created {slug}.");
        }
        SpaceCommand::Delete { slug } => {
            let url = client.url(&["api", "spaces", &slug])?;
            client.send(Method::DELETE, url, |r| r).await?;
            println!("Deleted {slug}.");
        }
        SpaceCommand::Members => {
            let slug = space(chosen, client)?;
            let members: Vec<SpaceMember> =
                client.get(&["api", "spaces", &slug, "members"]).await?;
            if json_out {
                println!("{}", pretty(&members)?);
            } else {
                for one in &members {
                    println!("{}  {}", one.email, one.role);
                }
            }
        }
        SpaceCommand::Use { slug } => {
            client.config.space = Some(slug.clone());
            client.config.save()?;
            println!("Now using {slug}.");
        }
    }
    Ok(())
}

async fn run(cli: Cli) -> Result<(), String> {
    let mut client = Client::new(Config::load()?);
    let json_out = cli.json;
    match cli.command {
        Command::Login { server, name } => {
            let server = server
                .or_else(|| client.config.server.clone())
                .unwrap_or_else(|| DEFAULT_SERVER.to_owned());
            let name = name.unwrap_or_else(login::machine_name);
            login::login(&mut client.config, &server, &name).await?;
        }
        Command::Logout => {
            client.config.access_token = None;
            client.config.refresh_token = None;
            client.config.save()?;
            println!("Logged out.");
        }
        Command::Whoami => {
            let me: Me = client.get(&["api", "me"]).await?;
            println!("{}", me.email);
        }
        Command::Space(command) => {
            space_command(&mut client, &cli.space, command, json_out).await?;
        }
        Command::Guide => {
            let url = client.url(&["api", "guide"])?;
            let res = client.send(Method::GET, url, |r| r).await?;
            print!("{}", res.text().await.map_err(|e| e.to_string())?);
        }
        Command::List { tag } => {
            let slug = space(&cli.space, &client)?;
            let mut listing: Listing = client.get(&["api", "spaces", &slug, "index"]).await?;
            if let Some(tag) = &tag {
                listing.items.retain(|item| item.tags.contains(tag));
            }
            if json_out {
                println!("{}", pretty(&listing)?);
            } else {
                for item in &listing.items {
                    println!("{}  {}: {}", item.id, item.name, item.summary);
                }
            }
        }
        Command::Read { id } => {
            let slug = space(&cli.space, &client)?;
            let doc: ContentDoc = client
                .get(&["api", "spaces", &slug, "content", &id])
                .await?;
            if json_out {
                println!("{}", pretty(&doc)?);
            } else {
                print!("{}", doc.text);
            }
        }
        Command::Write {
            name,
            file,
            tags,
            file_type,
            cover,
        } => {
            let slug = space(&cli.space, &client)?;
            let text = match &file {
                Some(path) => read_file(path)?,
                None => read_stdin(&format!(
                    "Publishing {name}. Type the text, then press ctrl-d."
                ))?,
            };
            let (body, media) = upload_text(&mut client, &slug, text, file.as_deref(), &[]).await?;
            let cover = match &cover {
                Some(path) => Some(upload_cover(&mut client, &slug, path).await?),
                None => None,
            };
            let new = NewContent {
                name: Some(name),
                file_type,
                tags,
                body,
                media,
                cover,
            };
            let url = client.url(&["api", "spaces", &slug, "content"])?;
            let made: ContentItem = client.fetch(Method::POST, url, |r| r.json(&new)).await?;
            if json_out {
                println!("{}", pretty(&made)?);
            } else {
                println!("Published {} as {}.", made.name, made.id);
            }
        }
        Command::Update {
            id,
            file,
            name,
            tags,
            cover,
            no_cover,
        } => {
            let slug = space(&cli.space, &client)?;
            // Read before taking the new text, so the version sent covers the
            // whole time that text was being written and anyone else's change
            // in that time is refused instead of lost.
            let doc: ContentDoc = client
                .get(&["api", "spaces", &slug, "content", &id])
                .await?;
            let text = match &file {
                Some(path) => Some(read_file(path)?),
                // With nothing else named, the text is what is being changed.
                None if name.is_none() && tags.is_none() && cover.is_none() && !no_cover => {
                    Some(read_stdin(&format!(
                        "Changing {}. Type the new text, then press ctrl-d.",
                        doc.item.name
                    ))?)
                }
                None => None,
            };
            let held: Vec<String> = doc.item.media.iter().map(|m| m.body.clone()).collect();
            let (body, media) = match text {
                Some(text) => {
                    let (body, media) =
                        upload_text(&mut client, &slug, text, file.as_deref(), &held).await?;
                    (Some(body), media)
                }
                None => (None, None),
            };
            let cover = match &cover {
                Some(path) => Some(upload_cover(&mut client, &slug, path).await?),
                None if no_cover => Some(String::new()),
                None => None,
            };
            let edit = ContentEdit {
                name,
                tags,
                body,
                media,
                cover,
                version: Some(doc.item.version),
                ..Default::default()
            };
            let url = client.url(&["api", "spaces", &slug, "content", &id])?;
            match client
                .fetch::<ContentItem>(Method::PUT, url, |r| r.json(&edit))
                .await
            {
                Ok(changed) => {
                    if json_out {
                        println!("{}", pretty(&changed)?);
                    } else {
                        println!("Changed {}.", changed.name);
                    }
                }
                Err(refused) => {
                    return Err(match refused.version {
                        Some(now) => format!(
                            "somebody changed this first, and it is now at version {now}. \
                             Read it again with `antropy read {id}`, then decide whether \
                             your change still applies."
                        ),
                        None => refused.message,
                    })
                }
            }
        }
        Command::Rm { id, yes } => {
            let slug = space(&cli.space, &client)?;
            let doc: ContentDoc = client
                .get(&["api", "spaces", &slug, "content", &id])
                .await?;
            if !yes && !agreed(&doc.item.name)? {
                println!("Left alone.");
                return Ok(());
            }
            // The version just read, so a change made meanwhile is refused.
            let version = doc.item.version.to_string();
            let url = client.url(&["api", "spaces", &slug, "content", &id])?;
            client
                .send(Method::DELETE, url, |r| r.query(&[("version", &version)]))
                .await?;
            println!("Removed {}.", doc.item.name);
        }
        Command::History { content } => {
            let slug = space(&cli.space, &client)?;
            let url = client.url(&["api", "spaces", &slug, "history"])?;
            let found: Vec<HistoryEntry> = client
                .fetch(Method::GET, url, |r| match &content {
                    Some(id) => r.query(&[("content", id)]),
                    None => r,
                })
                .await?;
            if json_out {
                println!("{}", pretty(&found)?);
            } else {
                for row in &found {
                    println!("#{}  {}  {}", row.version, row.who, row.what);
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(message) = run(Cli::parse()).await {
        eprintln!("error: {message}");
        std::process::exit(1);
    }
}
