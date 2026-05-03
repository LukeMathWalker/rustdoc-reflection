# `rustdoc_ir`

[![Crates.io](https://img.shields.io/crates/v/rustdoc_ir.svg)](https://crates.io/crates/rustdoc_ir)
[![Docs.rs](https://docs.rs/rustdoc_ir/badge.svg)](https://docs.rs/rustdoc_ir)

An Intermediate Representation (IR) for Rust types and functions, designed to
be the analysis-friendly counterpart to the raw types exposed by
[`rustdoc-types`](https://crates.io/crates/rustdoc-types).

The IR captures paths, references, tuples, slices, arrays, raw pointers,
function pointers, generics, and type aliases in a normalized form that's
straightforward to compare, render, and traverse. It's used by
[`rustdoc_resolver`](https://crates.io/crates/rustdoc_resolver) as the output
of converting `rustdoc-types` items into a stable, framework-agnostic shape.

Part of the [rustdoc-reflection](https://github.com/LukeMathWalker/rustdoc-reflection)
toolkit.

## License

Licensed under the [Apache License, Version 2.0](../LICENSE-APACHE).
