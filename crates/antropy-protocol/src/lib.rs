//! The wire types the server and its apps share.
//!
//! Everything under `generated/` comes from `protocol/antropy.fl` by way of
//! fluorite. Do not edit it; change the schema and run `make types`, which
//! regenerates the Rust here and the Swift the apps use, so the two sides
//! cannot drift apart.
//!
//! Field names go on the wire as **camelCase**: a `file_type` here is
//! `fileType` in JSON. A snake_case key in a hand-written request body is
//! ignored rather than refused, so hand-written bodies are a bad idea.
//!
//! Stored data deliberately does not use these types. Storage keeps its own
//! shapes inside `antropy-content`, so renaming a field for the apps can never
//! silently rewrite what is already on disk.

pub mod antropy {
    // The generated constructors take one argument per field, which trips the
    // argument-count lint on the wider types. The allow belongs here because
    // regeneration deletes everything inside `generated/`.
    #[allow(clippy::too_many_arguments)]
    pub mod v1 {
        // `include!` rather than a module file, because the generator's
        // `clean` step deletes everything in its output directory.
        include!("generated/antropy/v1/mod.rs");
    }
}

pub use antropy::v1::*;
