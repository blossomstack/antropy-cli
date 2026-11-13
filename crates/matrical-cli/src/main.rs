//! Matrical command line, for people and agents.

mod client;
mod config;
mod login;
mod media;

use clap::{Parser, Subcommand};
use matrical_protocol::{
    ContentDoc, ContentEdit, ContentItem, HistoryEntry, Listing, Me, MediaRef, NewContent,
    NewSpace, SearchResults, SpaceMember, SpaceSummary, UploadFile, UploadLinks, UploadRequest,
};

const DEFAULT_SERVER: &str = "https://api.matrical.app";

/// Shown under the long help, so an agent reading `--help` is given lines that
/// work rather than only a description of each one.
const EXAMPLES: &str = "\
Examples:
  matrical login --name \"build box\"  log in, and name the token
  matrical space use team            work in that space from now on (a name or an id)
  matrical search pricing tiers      best matches first, with the passage
  matrical list --tag research       ids and summaries, narrowed by tag
  matrical read 7f3a2b               the whole text of one item
  matrical write \"Pricing study\" --file study.md --tags research,pricing --cover chart.png
  matrical update 7f3a2b --file study.md
  matrical write \"Launch demo\" --video demo.mp4 --file notes.md --cover still.png
  matrical rm 7f3a2b --yes           remove it without being asked
  matrical list --json               the index as JSON, to pick apart
";
use client::Client;
use config::Config;
use reqwest::Method;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Publish content to a space, and read what is there.
///
/// Matrical is where work written by any AI agent lands and people read it. If
/// you are an agent, this help is the whole of how to use it; `matrical guide`
/// adds the conventions for publishing, from whoever runs the server.
///
/// A space is one shared folder of content. Choose one by its name or its id
/// with `matrical space use <name or id>`, or pass `--space <name or id>` to a
/// single command; `matrical space list` shows the ones you can reach, each
/// with its id. Spaces of different owners can share a name, and then only the
/// id will do. `matrical whoami` says whether this machine is
/// logged in at all, and `matrical login` is what fixes it if not.
///
/// To read: `matrical search <words>` is the quickest way to what you are after,
/// best match first. `matrical list` gives every item with its id and a one-line
/// summary, `--tag <tag>` narrows it, and `matrical read <id>` prints the whole
/// text of one. `--json` on any command prints the raw answer instead, which is
/// easier to pick apart than the lines meant for people.
///
/// To write: `matrical write "<name>" --file <path>` publishes, and it is live
/// the moment it goes in — nothing is held for review. `matrical update <id>
/// --file <path>` changes what is already there, and is refused if anybody
/// changed it since you read it: read it again and decide whether your change
/// still applies, rather than retrying. Nothing is ever truly erased, so never
/// publish a secret, a password or a key.
///
/// Give everything you publish a cover with --cover whenever you can: people
/// browse a space as tiles, and the cover is what they see first. Audio and
/// video above all, since without one their tile is only their name.
///
/// Two things hang an agent that has no terminal. `write` and `update` wait for
/// text on standard input when given no --file with nothing piped in, and `rm`
/// asks before removing unless passed --yes.
///
/// The login, the server address and the chosen space are kept in
/// ~/.config/matrical/config.json. Set MATRICAL_CONFIG to use a different file,
/// which is how two agents on one machine each get a login and a space of their
/// own.
#[derive(Parser)]
#[command(
    name = "matrical",
    version,
    arg_required_else_help = true,
    after_long_help = EXAMPLES
)]
struct Cli {
    /// Space to use, by name or id, instead of the one set with `space use`.
    #[arg(long, global = true)]
    space: Option<String>,
    /// Print the raw JSON answer, instead of the lines meant for people.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Log in through your browser.
    ///
    /// Two ways in are offered at once and whichever finishes first wins: the
    /// browser on this machine, or a QR code to scan with a phone. The phone is
    /// the way in over SSH, or on a machine with no browser of its own. It shows
    /// a code, which should be allowed only if it matches the one printed here.
    /// Set MATRICAL_NO_BROWSER to stop a browser being opened on this machine.
    ///
    /// Each login is a token of its own, named after this machine or after
    /// --name. In Settings, on the web or in the iPhone app, it can be renamed,
    /// limited to some spaces to read or to write, or revoked. An agent is
    /// better given a login of its own, limited to the spaces it should touch,
    /// than yours.
    ///
    /// The token is written to ~/.config/matrical/config.json, readable only by
    /// you. A person has to be there for a login: there is no password or key to
    /// pass in. So an unattended agent either logs in once on the machine it
    /// runs on, or is handed a config file from somewhere a person did.
    Login {
        /// Address of the server. Defaults to the one you last used, or
        /// https://api.matrical.app.
        #[arg(long)]
        server: Option<String>,
        /// What to call this login in your list of CLI tokens. Defaults to
        /// this machine's name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Forget this machine's login.
    ///
    /// Only this machine forgets. The token itself lives on until it is revoked
    /// in Settings.
    Logout,
    /// Show who you're logged in as.
    ///
    /// Prints the email address this machine's token belongs to, and fails when
    /// there is no working login. It is the cheapest way to check a setup before
    /// anything is published.
    Whoami,
    /// Manage spaces.
    ///
    /// A space is one shared folder of content, with its own members and its own
    /// conventions. `space use` writes the choice to the config file, so later
    /// commands need no --space; --space on a single command wins over it.
    #[command(subcommand)]
    Space(SpaceCommand),
    /// List everything in the space, each with a one-line summary.
    ///
    /// The id each line starts with is what `read`, `update` and `rm` take.
    /// --json gives the whole index instead, tags and summaries included.
    List {
        /// Show only what carries this tag.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Find content by what it says, best first, with the passage that matched.
    ///
    /// Where search by meaning is on, it also finds content that says the
    /// same thing in other words.
    Search {
        /// The words to look for.
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// How many results to show, at most 50.
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Match the words only, the meaning only, or both.
        #[arg(long, default_value = "both", value_parser = ["words", "meaning", "both"])]
        mode: String,
    },
    /// Print one piece of content.
    ///
    /// Prints the text as it was published. --json gives its name, tags, media
    /// and version as well.
    Read {
        /// The id that `list` shows.
        id: String,
    },
    /// Publish something new, from a file or standard input.
    ///
    /// With no --file and nothing piped in, it waits for the text on standard
    /// input, which hangs an agent that has no terminal.
    ///
    /// Start the text with a `# Title`, then one sentence saying what it is:
    /// that sentence is the summary people see in the list, so make it the
    /// finding rather than the method. Tags are how a long list gets narrowed,
    /// so use the few that somebody looking for this would guess.
    ///
    /// Images the markdown shows by a path next to it, such as
    /// `![Chart](./chart.png)`, are sent with it.
    ///
    /// --audio or --video publishes a recording instead: an MP3 or M4A up to
    /// 200 MB, or an MP4, MOV or WebM up to 1 GB. Its description is optional
    /// markdown from --file; standard input is never read for one.
    ///
    /// Add --cover whenever you have a picture, and always for audio and video:
    /// it leads the item's tile, the page above its player, and an iPhone's
    /// lock screen while it plays. A still from the video, the slide being
    /// discussed or a chart from the text all work.
    Write {
        /// What people will see in the list.
        name: String,
        /// Take the text from this file instead of standard input.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Tags, separated by commas.
        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
        /// Publish this MP3 or M4A as an audio item.
        #[arg(long, conflicts_with = "video")]
        audio: Option<PathBuf>,
        /// Publish this MP4, MOV or WebM as a video item.
        #[arg(long)]
        video: Option<PathBuf>,
        /// A PNG, JPEG, GIF or WebP to lead its tile; add one whenever you
        /// can, and always for audio and video. Without one, the first image
        /// the text shows is used, and failing that its tile is just words.
        #[arg(long)]
        cover: Option<PathBuf>,
        /// Who to credit, such as the agent writing it. Without it, the name
        /// this login has in Settings.
        #[arg(long)]
        author: Option<String>,
    },
    /// Change something already published.
    ///
    /// New text comes from --file, or from standard input when nothing else is
    /// being changed — which hangs an agent that has no terminal.
    ///
    /// The item is read first and the change carries the version read, so a
    /// change somebody else made meanwhile is refused rather than lost. When
    /// that happens, read the item again and decide whether your change still
    /// applies; retrying the same change cannot work.
    ///
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
        /// A PNG, JPEG, GIF or WebP to lead its tile. Worth adding to anything
        /// published without one, audio and video above all.
        #[arg(long, conflicts_with = "no_cover")]
        cover: Option<PathBuf>,
        /// Drop its chosen cover, so the first image its text shows is used.
        #[arg(long)]
        no_cover: bool,
        /// Replace the recording an audio item plays, or make it one.
        #[arg(long, conflicts_with = "video")]
        audio: Option<PathBuf>,
        /// Replace the recording a video item plays, or make it one. People
        /// see it as unread again.
        #[arg(long)]
        video: Option<PathBuf>,
    },
    /// Remove one piece of content.
    ///
    /// It goes from the list, but it is not erased: every version of everything
    /// published is kept for good. Without --yes it asks on the terminal first,
    /// so an unattended run has to pass --yes.
    Rm {
        /// The id that `list` shows.
        id: String,
        /// Go ahead without asking.
        #[arg(long)]
        yes: bool,
    },
    /// Show what has changed, newest first.
    ///
    /// One line per change: the version, who made it, and what they did.
    History {
        /// Follow one piece of content, by id, instead of the whole space.
        #[arg(long)]
        content: Option<String>,
    },
    /// Print instructions for agents.
    ///
    /// Fetched from the server and written by whoever runs it, so it is the
    /// authority on what belongs in a space and how to write it. Read it once
    /// before publishing anything. It needs a login.
    Guide,
}

#[derive(Subcommand)]
enum SpaceCommand {
    /// List your spaces, each with its id.
    List,
    /// Create a space, and print its id.
    Create {
        /// A short name: lowercase letters, numbers and dashes.
        slug: String,
        /// What people see. Defaults to the short name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Delete a space and everything in it.
    Delete {
        /// The space's short name or its id.
        space: String,
    },
    /// Show who can reach the space, and what they may do.
    Members,
    /// Set the space later commands use.
    Use {
        /// The space's short name or its id. Spaces of different owners can
        /// share a name; pass the id when they do.
        space: String,
    },
}

/// The id of the space chosen with --space or `space use`.
async fn space(chosen: &Option<String>, client: &mut Client) -> Result<String, String> {
    let wanted = chosen
        .clone()
        .or_else(|| client.config.space.clone())
        .ok_or_else(|| {
            "no space chosen: pick one with `matrical space use <name or id>`, or pass --space"
                .to_owned()
        })?;
    resolve(client, &wanted).await
}

/// Turns a space's name or id into its id. An id is taken as it is, without
/// asking the server.
async fn resolve(client: &mut Client, wanted: &str) -> Result<String, String> {
    if is_space_id(wanted) {
        return Ok(wanted.to_owned());
    }
    let spaces: Vec<SpaceSummary> = client.get(&["api", "spaces"]).await?;
    pick(&spaces, wanted)
}

/// An id is 32 lowercase hex characters; a short name never is.
fn is_space_id(wanted: &str) -> bool {
    wanted.len() == 32
        && wanted
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn pick(spaces: &[SpaceSummary], wanted: &str) -> Result<String, String> {
    let found: Vec<&SpaceSummary> = spaces.iter().filter(|s| s.slug == wanted).collect();
    match found.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => Err(format!(
            "no space named {wanted}: `matrical space list` shows the ones you can reach"
        )),
        many => {
            let lines: Vec<String> = many
                .iter()
                .map(|s| format!("  {}  {}", s.id, s.name))
                .collect();
            Err(format!(
                "{} spaces are named {wanted}; pass the id of the one you mean:\n{}",
                many.len(),
                lines.join("\n")
            ))
        }
    }
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

/// The kind of item and its file, when it plays a recording.
fn recording_arg(
    audio: Option<PathBuf>,
    video: Option<PathBuf>,
) -> Option<(&'static str, PathBuf)> {
    audio
        .map(|path| ("audio", path))
        .or(video.map(|path| ("video", path)))
}

/// Uploads a file and hands back its hash.
async fn upload_file(client: &mut Client, space_id: &str, path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hashes = upload(client, space_id, vec![bytes]).await?;
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
    space_id: &str,
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
    let url = client.url(&["api", "spaces", space_id, "uploads"])?;
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
    space_id: &str,
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
    let mut hashes = upload(client, space_id, files).await?.into_iter();
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
                    println!("{}  {}  {}  ({})", one.id, one.slug, one.name, one.role);
                }
            }
        }
        SpaceCommand::Create { slug, name } => {
            let body = NewSpace { slug, name };
            let url = client.url(&["api", "spaces"])?;
            let made: SpaceSummary = client.fetch(Method::POST, url, |r| r.json(&body)).await?;
            if json_out {
                println!("{}", pretty(&made)?);
            } else {
                println!("Created {} as {}.", made.slug, made.id);
            }
        }
        SpaceCommand::Delete { space } => {
            let space_id = resolve(client, &space).await?;
            let url = client.url(&["api", "spaces", &space_id])?;
            client.send(Method::DELETE, url, |r| r).await?;
            println!("Deleted {space}.");
        }
        SpaceCommand::Members => {
            let space_id = space(chosen, client).await?;
            let members: Vec<SpaceMember> =
                client.get(&["api", "spaces", &space_id, "members"]).await?;
            if json_out {
                println!("{}", pretty(&members)?);
            } else {
                for one in &members {
                    println!("{}  {}", one.email, one.role);
                }
            }
        }
        SpaceCommand::Use { space } => {
            let space_id = resolve(client, &space).await?;
            client.config.space = Some(space_id.clone());
            client.config.save()?;
            if space_id == space {
                println!("Now using {space_id}.");
            } else {
                println!("Now using {space} ({space_id}).");
            }
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
            let space_id = space(&cli.space, &mut client).await?;
            let mut listing: Listing = client.get(&["api", "spaces", &space_id, "index"]).await?;
            if let Some(tag) = &tag {
                listing.items.retain(|item| item.tags.contains(tag));
            }
            if json_out {
                println!("{}", pretty(&listing)?);
            } else {
                for item in &listing.items {
                    let kind = match item.file_type.as_str() {
                        "audio" | "video" => format!("[{}] ", item.file_type),
                        _ => String::new(),
                    };
                    println!("{}  {kind}{}: {}", item.id, item.name, item.summary);
                }
            }
        }
        Command::Search { query, limit, mode } => {
            let space_id = space(&cli.space, &mut client).await?;
            let url = client.url(&["api", "spaces", &space_id, "search"])?;
            let query = query.join(" ");
            let limit = limit.to_string();
            let found: SearchResults = client
                .fetch(Method::GET, url, |r| {
                    r.query(&[("q", &query), ("limit", &limit), ("mode", &mode)])
                })
                .await?;
            if json_out {
                println!("{}", pretty(&found)?);
            } else {
                if found.indexed_version < found.version {
                    eprintln!(
                        "Recent changes are still being indexed; they match by name only for now."
                    );
                }
                if found.hits.is_empty() {
                    eprintln!("Nothing matches.");
                }
                for hit in &found.hits {
                    let how = if hit.matched == "meaning" {
                        "  (similar meaning)"
                    } else {
                        ""
                    };
                    println!("{}  {}{how}", hit.item.id, hit.item.name);
                    if !hit.snippet.is_empty() {
                        println!("    {}", hit.snippet);
                    }
                }
            }
        }
        Command::Read { id } => {
            let space_id = space(&cli.space, &mut client).await?;
            let doc: ContentDoc = client
                .get(&["api", "spaces", &space_id, "content", &id])
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
            cover,
            author,
            audio,
            video,
        } => {
            let space_id = space(&cli.space, &mut client).await?;
            let playing = recording_arg(audio, video);
            let text = match &file {
                Some(path) => read_file(path)?,
                None if playing.is_some() => String::new(),
                None => read_stdin(&format!(
                    "Publishing {name}. Type the text, then press ctrl-d."
                ))?,
            };
            let (file_type, recording) = match playing {
                Some((kind, path)) => (
                    Some(kind.to_owned()),
                    Some(upload_file(&mut client, &space_id, &path).await?),
                ),
                // Everything else is markdown, which the server assumes.
                None => (None, None),
            };
            let (body, media) =
                upload_text(&mut client, &space_id, text, file.as_deref(), &[]).await?;
            let cover = match &cover {
                Some(path) => Some(upload_file(&mut client, &space_id, path).await?),
                None => None,
            };
            let new = NewContent {
                name: Some(name),
                file_type,
                tags,
                body,
                media,
                cover,
                recording,
                author,
            };
            let url = client.url(&["api", "spaces", &space_id, "content"])?;
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
            audio,
            video,
        } => {
            let playing = recording_arg(audio, video);
            let space_id = space(&cli.space, &mut client).await?;
            // Read before taking the new text, so the version sent covers the
            // whole time that text was being written and anyone else's change
            // in that time is refused instead of lost.
            let doc: ContentDoc = client
                .get(&["api", "spaces", &space_id, "content", &id])
                .await?;
            let text = match &file {
                Some(path) => Some(read_file(path)?),
                // With nothing else named, the text is what is being changed.
                None if name.is_none()
                    && tags.is_none()
                    && cover.is_none()
                    && !no_cover
                    && playing.is_none() =>
                {
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
                        upload_text(&mut client, &space_id, text, file.as_deref(), &held).await?;
                    (Some(body), media)
                }
                None => (None, None),
            };
            let cover = match &cover {
                Some(path) => Some(upload_file(&mut client, &space_id, path).await?),
                None if no_cover => Some(String::new()),
                None => None,
            };
            let (file_type, recording) = match playing {
                Some((kind, path)) => (
                    Some(kind.to_owned()),
                    Some(upload_file(&mut client, &space_id, &path).await?),
                ),
                None => (None, None),
            };
            let edit = ContentEdit {
                name,
                tags,
                body,
                media,
                cover,
                file_type,
                recording,
                version: Some(doc.item.version),
                // Ticks people made since are refused too, not overwritten.
                minor_version: Some(doc.item.minor_version),
            };
            let url = client.url(&["api", "spaces", &space_id, "content", &id])?;
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
                    return Err(match (refused.version, refused.minor_version) {
                        (Some(now), Some(minor)) if now == doc.item.version => format!(
                            "a person changed this first, such as by ticking a task: it now has \
                             {minor} such change(s) on version {now}. Read it again with \
                             `matrical read {id}`, then decide whether your change still applies."
                        ),
                        (Some(now), _) => format!(
                            "somebody changed this first, and it is now at version {now}. \
                             Read it again with `matrical read {id}`, then decide whether \
                             your change still applies."
                        ),
                        (None, _) => refused.message,
                    })
                }
            }
        }
        Command::Rm { id, yes } => {
            let space_id = space(&cli.space, &mut client).await?;
            let doc: ContentDoc = client
                .get(&["api", "spaces", &space_id, "content", &id])
                .await?;
            if !yes && !agreed(&doc.item.name)? {
                println!("Left alone.");
                return Ok(());
            }
            // The version just read, so a change made meanwhile is refused.
            let version = doc.item.version.to_string();
            let url = client.url(&["api", "spaces", &space_id, "content", &id])?;
            client
                .send(Method::DELETE, url, |r| r.query(&[("version", &version)]))
                .await?;
            println!("Removed {}.", doc.item.name);
        }
        Command::History { content } => {
            let space_id = space(&cli.space, &mut client).await?;
            let url = client.url(&["api", "spaces", &space_id, "history"])?;
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

#[cfg(test)]
mod tests {
    use super::{is_space_id, pick, Cli, DEFAULT_SERVER};
    use clap::CommandFactory;
    use matrical_protocol::SpaceSummary;

    fn summary(id: &str, slug: &str) -> SpaceSummary {
        SpaceSummary {
            id: id.to_owned(),
            slug: slug.to_owned(),
            name: slug.to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn only_32_lowercase_hex_is_taken_as_an_id() {
        assert!(is_space_id("0123456789abcdef0123456789abcdef"));
        assert!(!is_space_id("0123456789ABCDEF0123456789ABCDEF"));
        assert!(!is_space_id("0123456789abcdef0123456789abcde"));
        assert!(!is_space_id("team"));
    }

    #[test]
    fn a_name_picks_its_one_space_and_refuses_none_or_several() {
        let a = "a".repeat(32);
        let b = "b".repeat(32);
        let spaces = [
            summary(&a, "team"),
            summary(&b, "team"),
            summary(&b, "solo"),
        ];
        assert_eq!(pick(&spaces, "solo"), Ok(b.clone()));
        assert!(pick(&spaces, "nobody")
            .unwrap_err()
            .contains("no space named"));
        let several = pick(&spaces, "team").unwrap_err();
        assert!(several.contains(&a) && several.contains(&b), "{several}");
    }

    /// Every command's long help, run together: what an agent can read.
    fn all_help(command: &mut clap::Command) -> String {
        let mut text = command.render_long_help().to_string();
        let names: Vec<String> = command
            .get_subcommands()
            .map(|sub| sub.get_name().to_owned())
            .collect();
        for name in names {
            if let Some(sub) = command.find_subcommand_mut(&name) {
                text.push_str(&all_help(sub));
            }
        }
        text
    }

    fn sources() -> Vec<(String, String)> {
        let mut found = Vec::new();
        let mut left = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(dir) = left.pop() {
            for entry in std::fs::read_dir(dir).expect("the crate's own source") {
                let path = entry.expect("a readable entry").path();
                if path.is_dir() {
                    left.push(path);
                } else {
                    let text = std::fs::read_to_string(&path).expect("a readable file");
                    found.push((path.display().to_string(), text));
                }
            }
        }
        found
    }

    /// The help is all an agent gets, so a name the code reads from the
    /// environment has to appear in it. Scanned rather than listed, so a new
    /// one cannot be added without a word about it somewhere.
    #[test]
    fn the_help_names_every_environment_variable_the_code_reads() {
        let help = all_help(&mut Cli::command());
        for (file, text) in sources() {
            for (at, _) in text.match_indices("MATRICAL_") {
                let name: String = text[at..]
                    .chars()
                    .take_while(|c| c.is_ascii_uppercase() || *c == '_')
                    .collect();
                assert!(
                    help.contains(&name),
                    "{file} reads {name}, which no help text mentions"
                );
            }
        }
    }

    /// The server the CLI falls back to is written out in the help by hand, so
    /// a change to one without the other would send an agent to the wrong one.
    #[test]
    fn the_help_names_the_server_it_falls_back_to() {
        assert!(all_help(&mut Cli::command()).contains(DEFAULT_SERVER));
    }
}
