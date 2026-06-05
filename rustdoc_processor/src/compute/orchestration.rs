use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::Context;
use guppy::graph::{BuildTargetId, PackageGraph};
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use itertools::Itertools as _;
use serde::Deserialize;

use super::format::check_format;
use super::package_id_spec::PackageIdSpecification;
use super::progress::ComputeProgress;
use super::toolchain::get_toolchain_crate_docs;
use crate::TOOLCHAIN_CRATES;
use crate::rustdocflags;
use crate::utils::normalize_crate_name;

#[derive(Debug, thiserror::Error, Clone)]
#[error(
    "I failed to retrieve information about the public types of a package in your dependency tree ('{package_spec}')."
)]
pub struct CannotGetCrateData {
    pub package_spec: String,
    #[source]
    pub source: Arc<anyhow::Error>,
}

fn format_optional_version(v: &Option<Version>) -> Option<tracing::field::DisplayValue<String>> {
    v.as_ref().map(|v| {
        use std::fmt::Write;
        let mut s = format!("v{}.{}.{}", v.major, v.minor, v.patch);
        if !v.pre.is_empty() {
            write!(&mut s, "-{}", v.pre).unwrap();
        }
        tracing::field::display(s)
    })
}

/// Returns the library target name for a package, falling back to the
/// normalized package name if no lib target exists.
///
/// `cargo doc` names its output files after the lib target name, not the package name.
/// When a `Cargo.toml` sets `[lib] name = "foo"`, the output file is `foo.json`,
/// regardless of the package name.
fn lib_target_name(package_id: &PackageId, package_graph: &PackageGraph) -> String {
    if let Ok(metadata) = package_graph.metadata(package_id)
        && let Some(target) = metadata.build_target(&BuildTargetId::Library)
    {
        return target.name().to_owned();
    }
    // Fallback: normalize the package name (hyphens → underscores), which
    // is what cargo uses by default when no explicit [lib] name is set.
    normalize_crate_name(package_graph.metadata(package_id).unwrap().name())
}

/// Return the JSON documentation for one or more crates.
/// Crates are singled out, within the current workspace, using a `PackageIdSpecification`.
///
/// The documentation is computed on the fly for crates that are local to the current workspace.
/// The documentation is retrieved via `rustup` for toolchain crates (e.g. `std`).
pub fn compute_crate_docs<I, P>(
    toolchain_name: &str,
    package_graph: &PackageGraph,
    package_ids: I,
    current_dir: &Path,
    progress: &P,
) -> Result<HashMap<PackageId, rustdoc_types::Crate>, anyhow::Error>
where
    I: Iterator<Item = PackageId>,
    P: ComputeProgress + ?Sized,
{
    let mut to_be_computed = vec![];
    let mut results = HashMap::new();
    for package_id in package_ids {
        // Some crates are not compiled as part of the dependency tree of the current workspace.
        // They are instead bundled as part of Rust's toolchain and automatically available for import
        // and usage in your crate: the standard library (`std`), `core` (a smaller subset of `std`
        // that doesn't require an allocator), `alloc` (a smaller subset of `std` that assumes you
        // can allocate).
        // Since those crates are pre-compiled (and somewhat special), we can't generate their
        // documentation on the fly. We assume that their JSON docs have been pre-computed and are
        // available for us to look at.
        if TOOLCHAIN_CRATES.contains(&package_id.repr()) {
            let krate = get_toolchain_crate_docs(package_id.repr(), toolchain_name)?;
            results.insert(package_id, krate);
            continue;
        }

        let package_spec = PackageIdSpecification::from_package_id(&package_id, package_graph)?;
        to_be_computed.push((package_id, package_spec));
    }

    if to_be_computed.is_empty() {
        return Ok(results);
    }

    // We need to chunk the crates into batches, because `cargo rustdoc` can only compute the
    // documentation for multiple crates at once if all the crate names are unique within the
    // batch.
    //
    // That's due to the output naming scheme of `rustdoc`: the output file is `{crate_name}.json`.
    // If we were to pass multiple crates with the same name to `cargo rustdoc`, the output file
    // would be overwritten multiple times, and we would only be left with the documentation for
    // the last crate.
    let chunks = {
        let mut chunks: Vec<Vec<(PackageId, PackageIdSpecification)>> = vec![];
        let mut chunk_id2names = HashMap::<usize, IndexSet<_>>::new();
        'outer: for (package_id, package_spec) in to_be_computed {
            let target_name = lib_target_name(&package_id, package_graph);
            for (index, chunk) in chunks.iter_mut().enumerate() {
                let chunk_names = chunk_id2names.get_mut(&index).unwrap();
                if chunk_names.insert(target_name.clone()) {
                    chunk.push((package_id, package_spec));
                    continue 'outer;
                }
            }
            // We need a new chunk!
            let mut names = IndexSet::new();
            names.insert(target_name);
            chunk_id2names.insert(chunks.len(), names);
            chunks.push(vec![(package_id, package_spec)]);
        }
        chunks
    };

    let target_directory = package_graph.workspace().target_directory().as_std_path();
    for chunk in chunks {
        let chunk_package_ids: Vec<_> = chunk.iter().map(|(id, _)| id.clone()).collect();
        progress.before_computing(package_graph, &chunk_package_ids);
        let timer = std::time::Instant::now();

        let outcome = _compute_crate_docs(
            toolchain_name,
            chunk.iter().map(|(_, spec)| spec),
            current_dir,
        );

        let duration = timer.elapsed();
        progress.after_computed(package_graph, &chunk_package_ids, duration);

        outcome?;

        // It takes a while to deserialize the JSON output of `cargo doc`, so we parallelize
        // that part.
        use rayon::prelude::{IntoParallelIterator, ParallelIterator};
        for (package_id, krate) in chunk
            .into_par_iter()
            .map(|(package_id, package_spec)| {
                let krate = load_json_docs(
                    target_directory,
                    &package_id,
                    package_graph,
                    &package_spec,
                    toolchain_name,
                );
                (package_id, krate)
            })
            .collect::<Vec<_>>()
        {
            results.insert(package_id, krate?);
        }
    }
    Ok(results)
}

#[tracing::instrument(skip_all, fields(package_id_specs, cmd))]
fn _compute_crate_docs<'a, I>(
    toolchain_name: &str,
    package_id_specs: I,
    current_dir: &Path,
) -> Result<(), anyhow::Error>
where
    I: Iterator<Item = &'a PackageIdSpecification>,
{
    let package_id_specs: Vec<_> = package_id_specs.collect();
    tracing::Span::current().record("package_id_specs", package_id_specs.iter().join(", "));

    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("run")
        .current_dir(current_dir)
        .arg(toolchain_name)
        .arg("cargo")
        .arg("doc")
        .arg("--no-deps")
        .arg("--lib")
        .arg("--output-format=json")
        .arg("--document-private-items")
        .arg("-q");
    for package_id_spec in &package_id_specs {
        cmd.arg("-p").arg(package_id_spec.to_string());
    }
    cmd.arg("-Zunstable-options");
    cmd.env("RUSTDOCFLAGS", rustdocflags().join(" "));

    tracing::Span::current().record("cmd", tracing::field::debug(&cmd));

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run `cargo doc`.\n{cmd:?}"))?;

    if !status.success() {
        anyhow::bail!(
            "An invocation of `cargo doc` exited with non-zero status code.\n{:?}",
            cmd
        );
    }
    Ok(())
}

/// Locate the per-version JSON file produced by `cargo doc --output-format=json`.
///
/// With `cargo doc --output-format=json`, the JSON output is written to a per-version
/// directory under the build dir: `<target>/debug/build/<package_name>-<HASH>/out/<lib_name>.json`.
/// Distinct hashes for distinct versions, persisted across invocations — so duplicate-named
/// crates don't clobber each other.
///
/// Multiple `<HASH>` directories can exist for the *same* version of a package (e.g., when
/// different feature sets or build contexts produced separate fingerprints). To pick the right
/// one we look at the sibling fingerprint files at
/// `<target>/debug/.fingerprint/<package_name>-<HASH>/`:
/// - `doc-lib-<lib_name>.json` carries the version in `local[0].Precalculated`; we keep only
///   candidates whose version matches the one we want.
/// - `invoked.timestamp`'s mtime says when cargo last touched that unit; among same-version
///   candidates we take the freshest, i.e. the one cargo wrote in the most recent run.
fn find_json_doc(
    target_directory: &Path,
    package_name: &str,
    lib_name: &str,
    expected_version: &Version,
) -> Result<PathBuf, anyhow::Error> {
    let debug_dir = target_directory.join("debug");
    let build_dir = debug_dir.join("build");
    let fingerprint_root = debug_dir.join(".fingerprint");
    let lib_json_filename = format!("{lib_name}.json");
    let fingerprint_filename = format!("doc-lib-{lib_name}.json");
    let dir_prefix = format!("{package_name}-");

    let mut candidates: Vec<(PathBuf, String)> = Vec::new();
    let read_dir = fs_err::read_dir(&build_dir).with_context(|| {
        format!(
            "Failed to read cargo build directory at `{}`",
            build_dir.display()
        )
    })?;
    for entry in read_dir {
        let entry = entry?;
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        let Some(hash) = dir_name.strip_prefix(&dir_prefix) else {
            continue;
        };
        let candidate = entry.path().join("out").join(&lib_json_filename);
        if candidate.is_file() {
            candidates.push((candidate, hash.to_owned()));
        }
    }

    if candidates.is_empty() {
        anyhow::bail!(
            "No JSON output found for `{package_name}` (lib `{lib_name}`) under `{}`. \
             Expected `cargo doc --output-format=json` to write to `<build>/<HASH>/out/{lib_name}.json`.",
            build_dir.display()
        );
    }

    let target_version = expected_version.to_string();
    let mut matches: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (candidate, hash) in candidates {
        let fingerprint_dir = fingerprint_root.join(format!("{dir_prefix}{hash}"));
        let Ok(file) = fs_err::File::open(fingerprint_dir.join(&fingerprint_filename)) else {
            continue;
        };
        let Ok(fingerprint) = serde_json::from_reader::<_, DocFingerprint>(BufReader::new(file))
        else {
            continue;
        };
        if fingerprint
            .local
            .first()
            .and_then(|l| l.precalculated.as_deref())
            == Some(&target_version)
        {
            matches.push((candidate, fingerprint_dir));
        }
    }

    match matches.len() {
        0 => anyhow::bail!(
            "No JSON output found for `{package_name}` v{expected_version} under `{}`. \
             None of the build-dir entries had a fingerprint at `{}` matching that version.",
            build_dir.display(),
            fingerprint_root.display(),
        ),
        1 => Ok(matches.into_iter().next().unwrap().0),
        _ => Ok(matches
            .into_iter()
            .max_by_key(|(_, fingerprint_dir)| {
                fs_err::metadata(fingerprint_dir.join("invoked.timestamp"))
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH)
            })
            .unwrap()
            .0),
    }
}

/// Subset of cargo's fingerprint file at `.fingerprint/<pkg>-<HASH>/doc-lib-<lib>.json` —
/// just enough to disambiguate multiple versions of the same crate.
#[derive(Deserialize)]
struct DocFingerprint {
    local: Vec<DocFingerprintLocal>,
}

#[derive(Deserialize)]
struct DocFingerprintLocal {
    #[serde(rename = "Precalculated")]
    precalculated: Option<String>,
}

#[tracing::instrument(
    skip_all,
    fields(
        crate.name = package_id_spec.name,
        crate.version = format_optional_version(&package_id_spec.version),
        crate.source = package_id_spec.source,
    )
)]
fn load_json_docs(
    target_directory: &Path,
    package_id: &PackageId,
    package_graph: &PackageGraph,
    package_id_spec: &PackageIdSpecification,
    toolchain_name: &str,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    let metadata = package_graph
        .metadata(package_id)
        .with_context(|| format!("`{}` doesn't appear in the package graph", package_id.repr()))?;
    let lib_name = lib_target_name(package_id, package_graph);
    let json_path = find_json_doc(
        target_directory,
        metadata.name(),
        &lib_name,
        metadata.version(),
    )?;

    let span = tracing::trace_span!("Read and deserialize JSON output");
    let guard = span.enter();
    let file = fs_err::File::open(&json_path).context(
        "Failed to open the file containing the output of a `cargo doc` invocation.",
    )?;
    let mut reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(&mut reader);
    // The documentation for some crates (e.g. typenum) causes a "recursion limit exceeded" when
    // deserializing their docs using the default recursion limit.
    deserializer.disable_recursion_limit();
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
    match rustdoc_types::Crate::deserialize(deserializer) {
        Ok(krate) => {
            drop(guard);
            Ok(krate)
        }
        Err(e) => {
            // Reset the reader to the beginning of the file.
            if reader.seek(SeekFrom::Start(0)).is_ok()
                && let Err(format_err) = check_format(reader)
            {
                return Err(format_err).with_context(|| {
                        format!(
                            "The JSON docs at `{}` are not in the expected format. \
                            Are you using the right version of the `nightly` toolchain, `{}`, to generate the JSON docs?",
                            json_path.display(), toolchain_name
                        )
                    });
            }

            Err(e).with_context(|| {
                format!(
                    "Failed to deserialize the output of a `cargo doc` invocation (`{}`)",
                    json_path.display()
                )
            })
        }
    }
}
