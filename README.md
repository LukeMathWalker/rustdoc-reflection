# Compile-time Rust Reflection Toolkit, Powered By `rustdoc-json`

Building blocks for analyzing Rust crates via [`rustdoc`'s JSON output](https://doc.rust-lang.org/nightly/nightly-rustc/rustdoc_json_types/).

## Crates

| Crate | Description |
|-------|-------------|
| [`rustdoc_ext`](rustdoc_ext) | Extension traits for `rustdoc-types`. |
| [`rustdoc_ir`](rustdoc_ir) | An Intermediate Representation (IR) for Rust types and functions. |
| [`rustdoc_processor`](rustdoc_processor) | Compute, cache, index, and query `rustdoc` JSON documentation. |
| [`rustdoc_resolver`](rustdoc_resolver) | Convert `rustdoc_types` items into `rustdoc_ir` types. |

## Origin

These crates were extracted from [Pavex][pavex], which uses them to power its compile-time
dependency-injection analysis.

## Testing Strategy

The crates in this repository are currently tested 
transitively via [Pavex's][pavex] and [cheadergen's][cheadergen] test suites.
A standalone test suite will follow sooner or later.

[pavex]: https://github.com/LukeMathWalker/pavex
[cheadergen]: https://github.com/LukeMathWalker/cheadergen

## License

Licensed under the [Apache License, Version 2.0](LICENSE-APACHE).
