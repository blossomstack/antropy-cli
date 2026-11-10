//! What the wire actually looks like. These are the facts both sides rely on.

use antropy_protocol::{CliToken, ContentEdit, ContentItem, FilePass, MediaInfo, NewContent};

#[test]
fn keys_go_out_as_camel_case() {
    let item = ContentItem {
        id: "abc".into(),
        name: "A name".into(),
        summary: "A summary".into(),
        file_type: "markdown".into(),
        author: "agent".into(),
        tags: vec!["one".into()],
        created_ms: 1,
        updated_ms: 2,
        bytes: 3,
        version: 4,
        content_version: 4,
        body: "hash".into(),
        media: vec![MediaInfo {
            media_type: "image/png".into(),
            ..Default::default()
        }],
        cover: None,
        recording: None,
        read: None,
        minor_version: 0,
        tasks: None,
    };
    let json = serde_json::to_string(&item).unwrap();
    for key in ["fileType", "createdMs", "updatedMs", "mediaType"] {
        assert!(json.contains(key), "{json}");
    }
    for key in ["file_type", "created_ms", "updated_ms", "media_type"] {
        assert!(!json.contains(key), "{json}");
    }
}

#[test]
fn even_a_small_shape_is_camel_case() {
    // The small shapes are where the rule slips: a single underscore in one of
    // them is a key a client silently never sees.
    let json = serde_json::to_string(&FilePass {
        pass: "p".into(),
        files_url: "https://x.test/files/t/s/".into(),
        expires_ms: 1,
    })
    .unwrap();
    assert!(
        json.contains("filesUrl") && json.contains("expiresMs"),
        "{json}"
    );
    assert!(!json.contains("files_url"), "{json}");
}

#[test]
fn optional_fields_may_be_left_out() {
    // A client sending only what it wants to change must be accepted, so a
    // missing optional field has to mean "leave it alone", not an error.
    let edit: ContentEdit = serde_json::from_str("{}").expect("an empty edit is valid");
    assert!(edit.name.is_none() && edit.body.is_none() && edit.version.is_none());

    let one: ContentEdit =
        serde_json::from_str(r#"{"name":"hello"}"#).expect("one field is enough");
    assert_eq!(one.name.as_deref(), Some("hello"));

    let new: NewContent =
        serde_json::from_str(r#"{"body":"abc"}"#).expect("the text alone is enough to publish");
    assert!(new.name.is_none() && new.file_type.is_none() && new.media.is_none());
}

#[test]
fn a_snake_case_key_is_ignored_rather_than_refused() {
    // Worth pinning down: it fails quietly, which is why hand-written request
    // bodies are a bad idea and the generated types exist.
    let edit: ContentEdit = serde_json::from_str(r#"{"file_type":"text"}"#).unwrap();
    assert!(edit.file_type.is_none(), "snake_case is silently dropped");
    let ok: ContentEdit = serde_json::from_str(r#"{"fileType":"text"}"#).unwrap();
    assert_eq!(ok.file_type.as_deref(), Some("text"));
}

#[test]
fn a_cli_token_is_camel_case_too() {
    let json = serde_json::to_string(&CliToken {
        all_spaces: true,
        ..Default::default()
    })
    .unwrap();
    for key in ["allSpaces", "lastUsedMs", "createdMs"] {
        assert!(json.contains(key), "{json}");
    }
    assert!(!json.contains("all_spaces"), "{json}");
}
