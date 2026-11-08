
/// One piece of content, without its text.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    pub id: String,
    /// What a person sees in a list.
    pub name: String,
    /// One line drawn from the start of the text.
    pub summary: String,
    /// markdown, text, json or html.
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
    /// The sha256 of the stored text, in hex. Fetch it with a file pass.
    pub body: String,
    /// The images and recordings markdown text shows, each stored beside it.
    pub media: Vec<crate::antropy::v1::MediaInfo>,
}

/// One image or recording a piece of markdown shows, as `![alt](name)`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// What the text calls it.
    pub name: String,
    /// The sha256 of the stored file, in hex. Fetch it with a file pass.
    pub body: String,
    /// image/png, image/jpeg, image/gif, image/webp, audio/mpeg or audio/mp4.
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
    pub space: String,
    pub version: u64,
    pub generated_ms: u64,
    pub items: Vec<crate::antropy::v1::ContentItem>,
}

/// What changed in a space after some version.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
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
/// file type is worked out from the text.
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
    /// The version the caller last read. The edit is refused if it has moved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

/// Who the caller is.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub email: String,
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

/// Why a request was refused.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, derive_new::new)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// A short code: unauthorized, forbidden, not_found, bad_request,
    /// conflict or internal.
    pub error: String,
    /// Wording a person can read.
    pub message: String,
    /// For a refused write, the version the content is at now, so the caller
    /// can read it again and decide rather than retrying blindly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}
