//! Shared local-first domain kernel. No network listener, UI, or cloud database.
pub mod chat;
pub mod companion;
pub mod event;
pub mod pairing;
pub mod persona;
pub mod ports;
pub mod relationship;
pub mod replication;
pub mod storage;
pub mod task;
pub mod video;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid input: {0}")]
    Invalid(&'static str),
    #[error("event identity or immutable entity conflict")]
    Conflict,
    #[error("untrusted or wrong-profile event origin")]
    Untrusted,
    #[error("stale, expired or unauthorized task lease")]
    StaleLease,
    #[error("unsupported schema or migration checksum mismatch")]
    Schema,
    #[error("database is already open by another application instance")]
    DatabaseInUse,
}
pub type Result<T> = std::result::Result<T, Error>;
