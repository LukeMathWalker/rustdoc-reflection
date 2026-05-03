# `rustdoc_ext`

[![Crates.io](https://img.shields.io/crates/v/rustdoc_ext.svg)](https://crates.io/crates/rustdoc_ext)
[![Docs.rs](https://docs.rs/rustdoc_ext/badge.svg)](https://docs.rs/rustdoc_ext)

Extension traits for [`rustdoc-types`](https://crates.io/crates/rustdoc-types).

`rustdoc_ext` keeps `rustdoc-types` close to upstream by providing utilities
(e.g. `ItemEnum -> ItemKind` mapping, human-readable descriptions of item
kinds, a `GlobalItemId` that disambiguates items across crates) as separate
extension traits rather than patching them into `rustdoc-types` itself.

Part of the [rustdoc-reflection](https://github.com/LukeMathWalker/rustdoc-reflection)
toolkit.

## License

Licensed under the [Apache License, Version 2.0](../LICENSE-APACHE).
