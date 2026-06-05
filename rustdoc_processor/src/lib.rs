//! Compute, cache, index, and query `rustdoc` JSON documentation for crates
//! in a project's dependency graph.
//!
//! The pipeline:
//!
//! 1. **Compute** ([`compute::compute_crate_docs`]): invoke `cargo doc --output-format=json`
//!    to generate JSON docs. Requires a nightly toolchain of `nightly-2026-06-04` or newer.
//! 2. **Cache** ([`cache::RustdocGlobalFsCache`]): persist raw docs and secondary indexes
//!    in a SQLite database at `{cache_dir}/{fingerprint}.db`.
//! 3. **Index** ([`indexing`]): build secondary indexes — import paths, item lookups,
//!    and external re-export tracking.
//! 4. **Query** ([`queries::Crate`], [`CrateCollection`]): look up items by path, resolve
//!    cross-crate references, and retrieve canonical import paths.

pub mod cache;
mod collection;
pub mod compute;
pub mod crate_data;
pub mod indexing;
pub mod queries;
mod unknown_item_path;
mod utils;
mod version_matcher;

// Cross-cutting types re-exported at crate root
pub use collection::CrateCollection;
pub use rustdoc_ext::GlobalItemId;
pub use unknown_item_path::UnknownItemPath;

/// Crate version - used as part of cache fingerprint.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Standard library crate package ID representation.
pub const STD_PACKAGE_ID_REPR: &str = "std";
/// Core crate package ID representation.
pub const CORE_PACKAGE_ID_REPR: &str = "core";
/// Alloc crate package ID representation.
pub const ALLOC_PACKAGE_ID_REPR: &str = "alloc";

/// The set of toolchain crates that are bundled with Rust.
pub const TOOLCHAIN_CRATES: [&str; 3] = [
    STD_PACKAGE_ID_REPR,
    CORE_PACKAGE_ID_REPR,
    ALLOC_PACKAGE_ID_REPR,
];

/// Return the options to pass to `rustdoc` via `RUSTDOCFLAGS` when invoking `cargo doc`.
///
/// Most options that used to live here have moved to native `cargo doc` flags
/// (`--document-private-items`, `--output-format=json`, `-Zunstable-options`). Only
/// `--document-hidden-items` remains, because cargo doesn't expose a passthrough for it.
///
/// We isolate this logic in a function so that the cache fingerprint (see the `cache`
/// module) reflects any future change to this list.
pub(crate) fn rustdocflags() -> [&'static str; 1] {
    ["--document-hidden-items"]
}
