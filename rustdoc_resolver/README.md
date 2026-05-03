# `rustdoc_resolver`

[![Crates.io](https://img.shields.io/crates/v/rustdoc_resolver.svg)](https://crates.io/crates/rustdoc_resolver)
[![Docs.rs](https://docs.rs/rustdoc_resolver/badge.svg)](https://docs.rs/rustdoc_resolver)

Convert items from
[`rustdoc-types`](https://crates.io/crates/rustdoc-types) into the IR exposed
by [`rustdoc_ir`](https://crates.io/crates/rustdoc_ir).

`rustdoc_resolver` sits between
[`rustdoc_processor`](https://crates.io/crates/rustdoc_processor) (input) and
`rustdoc_ir` (output) and provides the core type-resolution logic — resolving
through (or preserving) type aliases, substituting generic bindings, and
converting free functions and methods into IR callables — with no
framework-specific dependencies.

Part of the [rustdoc-reflection](https://github.com/LukeMathWalker/rustdoc-reflection)
toolkit.

## License

Licensed under the [Apache License, Version 2.0](../LICENSE-APACHE).
