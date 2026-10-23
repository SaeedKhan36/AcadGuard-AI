//! plagengine-core: plagiarism detection whose output mirrors Turnitin's
//! Enhanced Similarity Report.
//!
//! Pipeline (each stage is a pure, separately tested module):
//! [`extract`] → [`normalize`] → [`fingerprint`] → [`candidates`] → [`align`]
//! → [`attribute`] → [`report`].
#![forbid(unsafe_code)]

pub mod align;
pub mod attribute;
pub mod candidates;
pub mod context;
pub mod engine;
pub mod eval;
pub mod extract;
pub mod fingerprint;
pub mod flags;
pub mod normalize;
pub mod report;
pub mod store;
pub mod types;

pub use engine::{check, check_with_index, prepare, CheckOptions, CheckResult, Passage, PreparedDoc};
pub use report::{build_report, Filters, Report};
pub use store::{Store, StoreConfig};
pub use types::{Collection, DocFormat, ExtractedDoc, HiddenReason, HiddenSpan, Span};

/// Crate-wide error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error on {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("unsupported file format: {0}")]
    UnsupportedFormat(String),
    #[error("failed to extract {path}: {msg}")]
    Extract { path: String, msg: String },
    #[error("store error: {0}")]
    Store(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
