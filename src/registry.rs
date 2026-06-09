//! Metric registry: file → provider selection and metric dispatch.
//!
//! Spec: `docs/spec.md` → "Metric Registry and Providers". The registry implements
//! [`MetricResolver`] so the engine can resolve `(subject, path, metric)` triples. Providers are
//! tried in priority order (format-specific first, the generic fallback last) and selected when
//! they both `supports(path)` and `handles(metric)`. Constructed providers are cached per
//! `(provider kind, path)` so multiple metrics on one file reuse a single context/scan.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::engine::MetricResolver;
use crate::model::Value;
use crate::providers::{
    BamProvider, FastaProvider, FastqProvider, GenericFileProvider, MetricProvider, VcfProvider,
};

/// A registrable provider kind: how to detect, route, and construct a provider.
struct ProviderKind {
    name: &'static str,
    supports: fn(&Path) -> bool,
    handles: fn(&str) -> bool,
    construct: fn(&Path) -> Result<Box<dyn MetricProvider>>,
}

/// Routes metric lookups to the appropriate provider, caching constructed providers per file.
pub struct MetricRegistry {
    kinds: Vec<ProviderKind>,
    cache: HashMap<(usize, PathBuf), Box<dyn MetricProvider>>,
}

impl MetricRegistry {
    /// Build a registry with all built-in providers registered in priority order.
    ///
    /// Format-specific providers (added in Phase 5) are registered before the generic fallback
    /// so that format metrics route to them while generic metrics (`exists`, `size`, …) fall
    /// through to [`GenericFileProvider`].
    pub fn with_default_providers() -> Self {
        let kinds = vec![
            // Phase 5 format providers (specific first), then the generic fallback.
            ProviderKind {
                name: "fasta",
                supports: FastaProvider::supports,
                handles: FastaProvider::handles,
                construct: |path| Ok(Box::new(FastaProvider::new(path)?)),
            },
            ProviderKind {
                name: "fastq",
                supports: FastqProvider::supports,
                handles: FastqProvider::handles,
                construct: |path| Ok(Box::new(FastqProvider::new(path)?)),
            },
            ProviderKind {
                name: "vcf",
                supports: VcfProvider::supports,
                handles: VcfProvider::handles,
                construct: |path| Ok(Box::new(VcfProvider::new(path)?)),
            },
            ProviderKind {
                name: "bam",
                supports: BamProvider::supports,
                handles: BamProvider::handles,
                construct: |path| Ok(Box::new(BamProvider::new(path)?)),
            },
            ProviderKind {
                name: "generic",
                supports: GenericFileProvider::supports,
                handles: GenericFileProvider::handles,
                construct: |path| Ok(Box::new(GenericFileProvider::new(path)?)),
            },
        ];
        MetricRegistry {
            kinds,
            cache: HashMap::new(),
        }
    }

    /// Find the index of the first provider kind that supports `path` and handles `metric`.
    fn select_kind(&self, path: &Path, metric: &str) -> Result<usize> {
        self.kinds
            .iter()
            .position(|kind| (kind.supports)(path) && (kind.handles)(metric))
            .ok_or_else(|| {
                anyhow!(
                    "no provider handles metric `{metric}` for {}",
                    path.display()
                )
            })
    }
}

impl MetricResolver for MetricRegistry {
    fn resolve(&mut self, _subject: &str, path: &Path, metric: &str) -> Result<Value> {
        let idx = self.select_kind(path, metric)?;
        let key = (idx, path.to_path_buf());
        if !self.cache.contains_key(&key) {
            let provider = (self.kinds[idx].construct)(path)?;
            self.cache.insert(key.clone(), provider);
        }
        // Cache key is present; expect is safe.
        self.cache
            .get_mut(&key)
            .expect("provider just inserted")
            .get(metric)
    }
}

impl std::fmt::Debug for MetricRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.kinds.iter().map(|k| k.name).collect();
        f.debug_struct("MetricRegistry")
            .field("kinds", &names)
            .field("cached", &self.cache.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_file(contents: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(contents).expect("write");
        file.flush().expect("flush");
        file
    }

    #[test]
    fn resolves_generic_metrics() {
        let file = temp_file(b"hello\n");
        let mut registry = MetricRegistry::with_default_providers();
        assert_eq!(
            registry.resolve("data", file.path(), "exists").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            registry.resolve("data", file.path(), "size").unwrap(),
            Value::Integer(6)
        );
    }

    #[test]
    fn unhandled_metric_reports_clear_error() {
        let file = temp_file(b"hello\n");
        let mut registry = MetricRegistry::with_default_providers();
        let err = registry
            .resolve("bam", file.path(), "read_count")
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("no provider handles metric `read_count`"),
            "got: {err}"
        );
    }

    #[test]
    fn caches_provider_per_path() {
        let file = temp_file(b"hello\n");
        let mut registry = MetricRegistry::with_default_providers();
        let _ = registry.resolve("data", file.path(), "md5").unwrap();
        let _ = registry.resolve("data", file.path(), "sha256").unwrap();
        // One generic provider cached for this path.
        assert_eq!(registry.cache.len(), 1);
    }
}
