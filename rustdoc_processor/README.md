# `rustdoc_processor`

[![Crates.io](https://img.shields.io/crates/v/rustdoc_processor.svg)](https://crates.io/crates/rustdoc_processor)
[![Docs.rs](https://docs.rs/rustdoc_processor/badge.svg)](https://docs.rs/rustdoc_processor)

Compute, cache, index, and query `rustdoc` JSON documentation for crates in a
project's dependency graph.

The pipeline:

1. **Compute** — invoke `cargo rustdoc` to generate JSON docs.
2. **Cache** — persist raw docs and secondary indexes in a SQLite database.
3. **Index** — build secondary indexes (import paths, item lookups, external
   re-export tracking).
4. **Query** — look up items by path, resolve cross-crate references, and
   retrieve canonical import paths.

Part of the [rustdoc-reflection](https://github.com/LukeMathWalker/rustdoc-reflection)
toolkit.

## License

Licensed under the [Apache License, Version 2.0](../LICENSE-APACHE).
