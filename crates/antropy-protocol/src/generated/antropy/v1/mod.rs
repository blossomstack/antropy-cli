
/// One piece of content, without its text.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    pub id: String,
    /// What a person sees in a list.
    pub name: String,
    /// One line drawn from the start of the text.
    pub summary: String,
    /// markdown, text, json, html, audio or video. Audio and video play
    /// `recording`, and their text is an optional markdown description.
    pub file_type: String,
    /// The connection that published it, or a person's email address.
    pub author: String,
    pub tags: Vec<String>,
    pub created_ms: u64,
    pub updated_ms: u64,
    /// Length of the text in bytes.
    pub bytes: u64,
    /// Rises whenever this item changes. Send it back to edit safely.
    pub version: u64,
    /// Rises only when its text or recording changes, not its name, tags or
    /// cover. Unique
    /// within its space; pages are ordered by it, newest first.
    pub content_version: u64,
    /// The sha256 of the stored text, in hex. Read the text with
    /// `GET /api/spaces/{space}/content/{id}/text?body=<this>`, which adds the
    /// changes `minorVersion` counts; a file pass fetches it without them.
    pub body: String,
    /// The images and recordings markdown text shows, each stored beside it.
    pub media: Vec<crate::antropy::v1::MediaInfo>,
    /// The picture to show for this item in a grid: the one chosen for it,
    /// else the first image its text shows. None when it has neither.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<crate::antropy::v1::MediaInfo>,
    /// What an audio or video item plays. None for every other kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<crate::antropy::v1::MediaInfo>,
    /// Whether the person asking has read it since its text last changed.
    /// Left out for the CLI, and wherever nobody in particular is asking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<bool>,
    /// How many changes people have made on top of `version`, such as
    /// ticking a task, that are not saved into it yet. `body` already shows
    /// them. Send it back with `version` to edit safely. 0 when there are none.
    pub minor_version: u64,
    /// Its markdown's task list items, and how many are ticked. None when it
    /// has none, and for items not changed since counts began.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<crate::antropy::v1::TaskCount>,
}

/// How many of an item's tasks, `- [ ]` and `- [x]`, are ticked.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct TaskCount {
    pub done: u64,
    pub total: u64,
}

/// Ticks or unticks one task in an item's markdown. Only people can, in a
/// browser or the app.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct TaskTick {
    /// The line the task is on, counting from 1.
    pub line: u64,
    /// The task's words as written after its box, trimmed. If the text has
    /// changed and `line` holds another task, the one task with these words
    /// is ticked instead.
    pub text: String,
    pub done: bool,
}

/// One image or recording a piece of markdown shows, as `![alt](name)`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// What the text calls it.
    pub name: String,
    /// The sha256 of the stored file, in hex. Fetch it with a file pass.
    pub body: String,
    /// image/png, image/jpeg, image/gif, image/webp, audio/mpeg, audio/mp4,
    /// video/mp4, video/quicktime or video/webm.
    pub media_type: String,
    pub bytes: u64,
    pub alt: String,
}

/// Lets one caller fetch stored files for a while: every file in a space, or
/// one file for long enough to play it through.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct FilePass {
    /// Send as `Authorization: Bearer <pass>`, or as `?verify=<pass>`.
    pub pass: String,
    /// Add a `body` hash to this to get its address.
    pub files_url: String,
    /// Ask for a new pass before this.
    pub expires_ms: u64,
}

/// Files to be uploaded straight to storage before they are published.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct UploadRequest {
    pub files: Vec<crate::antropy::v1::UploadFile>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct UploadFile {
    /// The sha256 of the file, in hex.
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct UploadLinks {
    pub uploads: Vec<crate::antropy::v1::UploadLink>,
}

/// Where to PUT one file. Storage refuses any other bytes: send exactly
/// `bytes` of them, with `x-amz-checksum-sha256: <checksum>`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct UploadLink {
    pub sha256: String,
    pub url: String,
    /// The sha256 in base64, the form the header takes.
    pub checksum: String,
    /// Start the upload before this.
    pub expires_ms: u64,
}

/// A stored file the text names, uploaded beforehand.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct MediaRef {
    /// What the text calls it, as in `![alt](name)`.
    pub name: String,
    /// The sha256 of the file, in hex.
    pub body: String,
}

/// One piece of content together with its text.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ContentDoc {
    pub item: crate::antropy::v1::ContentItem,
    pub text: String,
}

/// Every item in a space, and the version the list reflects.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    /// The space's id.
    pub space: String,
    pub version: u64,
    pub generated_ms: u64,
    pub items: Vec<crate::antropy::v1::ContentItem>,
}

/// Part of a space, newest content first, with what the person asking has
/// read. Ask for the next part with `before` set to `next`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ContentPage {
    /// The space's id.
    pub space: String,
    /// The space's version when this was made.
    pub version: u64,
    pub generated_ms: u64,
    pub items: Vec<crate::antropy::v1::ContentItem>,
    /// The `contentVersion` to ask for items before. None at the end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<u64>,
}

/// Content versions from `from` to `to`, both included.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ReadRange {
    pub from: u64,
    pub to: u64,
}

/// What a person has read or wants to read again, by content version. Ranges
/// may cover versions no item holds any more; versions not given out yet are
/// never marked.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ReadMarks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<Vec<crate::antropy::v1::ReadRange>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread: Option<Vec<u64>>,
}

/// What changed in a space after some version.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    /// The space's id.
    pub space: String,
    pub version: u64,
    pub generated_ms: u64,
    /// The answer would have had a hole in it, so take the whole listing.
    pub full_refresh: bool,
    pub items: Vec<crate::antropy::v1::ContentItem>,
    /// Ids that are no longer there.
    pub removed: Vec<String>,
}

/// Content to publish, its text and media uploaded first. A missing name or
/// file type is worked out from the text; audio and video need a name.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct NewContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// The sha256 of the uploaded text, in hex.
    pub body: String,
    /// Every image or recording the text shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<crate::antropy::v1::MediaRef>>,
    /// The sha256 of an uploaded PNG, JPEG, GIF or WebP image to show for
    /// this item in a grid. Left out, the first image the text shows is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// The sha256 of the uploaded MP3 or M4A an audio item plays, or the MP4,
    /// MOV or WebM a video item plays. Required for those, refused otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<String>,
    /// Who to credit, such as the agent writing it. Left out, the CLI
    /// token's name, or the person's email in a browser or the app.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Fields to change. Anything left out keeps the value it has.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ContentEdit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// The sha256 of new uploaded text, in hex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Media the new text shows that the item does not have yet, or that
    /// replaces what it has under the same name. Needs `body`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<crate::antropy::v1::MediaRef>>,
    /// The sha256 of an uploaded image to show for this item in a grid. An
    /// empty string drops the chosen one, so the text's first image is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// The sha256 of a new uploaded recording for an audio or video item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<String>,
    /// The version the caller last read. The edit is refused if it has moved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    /// The minor version read along with `version`. The edit is refused if a
    /// person has changed the item since. Left out, it is 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor_version: Option<u64>,
}

/// Who the caller is.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub email: String,
}

/// Allows or denies a login a terminal started, by the code it shows. Only
/// the iPhone app and a browser may send it, never a CLI login.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAnswer {
    /// As the terminal shows it, with or without the dash.
    pub code: String,
    pub allow: bool,
}

/// A command-line login: the CLI on one machine, or an agent using it. Only
/// the iPhone app and a browser can see or change these, never a CLI login.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct CliToken {
    pub id: String,
    /// What it is called. Starts as the name of the machine that logged in.
    pub name: String,
    pub created_ms: u64,
    /// When it last renewed its sign-in, which it does each hour it is used.
    pub last_used_ms: u64,
    /// True when it reaches every space its owner can, as far as the owner's
    /// role allows. False when it reaches only `spaces`.
    pub all_spaces: bool,
    /// The spaces it may reach when it is limited to some.
    pub spaces: Vec<crate::antropy::v1::TokenSpace>,
}

/// One space a CLI token may reach, and how far. Never further than the
/// owner's own role.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct TokenSpace {
    /// The space's id.
    pub id: String,
    /// read or write.
    pub access: String,
}

/// Fields to change on a CLI token. Anything left out keeps its value.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct CliTokenEdit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_spaces: Option<bool>,
    /// Every space it may reach while limited. Replaces the list it had.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spaces: Option<Vec<crate::antropy::v1::TokenSpace>>,
}

/// Somebody who can reach a space, and what they may do.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct SpaceMember {
    pub email: String,
    /// reader, writer or owner.
    pub role: String,
}

/// A space to make. A missing name is taken from the slug.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct NewSpace {
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A space the caller can reach.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSummary {
    /// What every request names it by. It never changes.
    pub id: String,
    /// A short name, unique among its owner's spaces. Only for showing and
    /// for finding a space by name; requests use `id`.
    pub slug: String,
    pub name: String,
    /// reader, writer or owner.
    pub role: String,
}

/// One change to a space, newest first.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub version: u64,
    pub at_ms: u64,
    /// Who made the change.
    pub who: String,
    /// What they did, in words.
    pub what: String,
    /// Ids of the content this change touched.
    pub content: Vec<String>,
}

/// What a search of one space found, best first.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    /// The space's id.
    pub space: String,
    pub query: String,
    /// The space's version now.
    pub version: u64,
    /// The version the search indexes reflect. While it is behind `version`,
    /// the newest changes are found by name, summary and tags only; indexing
    /// catches up within seconds.
    pub indexed_version: u64,
    /// Whether this search matched by meaning as well as by words.
    pub meaning: bool,
    pub hits: Vec<crate::antropy::v1::SearchHit>,
}

/// One item a search found.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub item: crate::antropy::v1::ContentItem,
    /// Words from the text where it matched, without markup. Its summary when
    /// only the name or tags matched.
    pub snippet: String,
    /// words, meaning or both: which search found it.
    pub matched: String,
    /// For a match by meaning, how close it is, up to 1 for the same meaning.
    /// Only comparable within one search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
}

/// How much of its plan a tenant has used.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// free or paid.
    pub plan: String,
    /// Writes this calendar month, UTC: new items and new text. Renames, tags,
    /// covers and removals are free.
    pub writes: u64,
    pub writes_limit: u64,
    /// Bytes stored, including files nothing uses any more until they have
    /// been kept for `keep_days`.
    pub bytes: u64,
    pub bytes_limit: u64,
    /// How long replaced and removed content is kept before it is deleted.
    pub keep_days: u64,
    /// When storage was last measured whole. Writes since are added as they
    /// happen. None until the first measure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured_ms: Option<u64>,
}

/// Why a request was refused.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// A short code: unauthorized, forbidden, not_found, bad_request,
    /// conflict, write_limit, storage_full or internal.
    pub error: String,
    /// Wording a person can read.
    pub message: String,
    /// For a refused write, the version the content is at now, so the caller
    /// can read it again and decide rather than retrying blindly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    /// With `version`, the minor version the content is at now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minor_version: Option<u64>,
}
