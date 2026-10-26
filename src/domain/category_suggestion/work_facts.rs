use crate::domain::metadata_source::MetadataProvider;

/// One source's work type and tags for a work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceFacts {
    provider: MetadataProvider,
    work_type: Option<String>,
    tags: Vec<String>,
}

impl SourceFacts {
    pub(crate) fn new(
        provider: MetadataProvider,
        work_type: Option<String>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            provider,
            work_type,
            tags,
        }
    }

    pub(crate) fn provider(&self) -> MetadataProvider {
        self.provider
    }

    pub(crate) fn work_type(&self) -> Option<&str> {
        self.work_type.as_deref()
    }

    pub(crate) fn tags(&self) -> &[String] {
        &self.tags
    }
}

/// Everything a suggestion is derived from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkFacts {
    sources: Vec<SourceFacts>,
    legacy_locations: Vec<String>,
    old_primaries: Vec<String>,
}

impl WorkFacts {
    /// `legacy_locations`: the top-level folder of each of the work's files. `old_primaries`:
    /// old per-file category values ("Visual Novel", "Action", …).
    pub(crate) fn new(
        sources: Vec<SourceFacts>,
        legacy_locations: Vec<String>,
        old_primaries: Vec<String>,
    ) -> Self {
        Self {
            sources,
            legacy_locations,
            old_primaries,
        }
    }

    pub(crate) fn sources(&self) -> &[SourceFacts] {
        &self.sources
    }

    pub(crate) fn legacy_locations(&self) -> &[String] {
        &self.legacy_locations
    }

    pub(crate) fn old_primaries(&self) -> &[String] {
        &self.old_primaries
    }
}
