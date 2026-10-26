use crate::domain::metadata_source::MetadataProvider;

/// The work's id on each site it is linked to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExternalIds {
    dlsite: Option<String>,
    vndb: Option<String>,
    steam: Option<String>,
}

impl ExternalIds {
    pub(crate) fn none() -> Self {
        Self::default()
    }

    pub(crate) fn new(dlsite: Option<String>, vndb: Option<String>, steam: Option<String>) -> Self {
        Self {
            dlsite,
            vndb,
            steam,
        }
    }

    pub(crate) fn with(mut self, provider: MetadataProvider, id: String) -> Self {
        match provider {
            MetadataProvider::Dlsite => self.dlsite = Some(id),
            MetadataProvider::Vndb => self.vndb = Some(id),
            MetadataProvider::Steam => self.steam = Some(id),
        }
        self
    }

    pub(crate) fn get(&self, provider: MetadataProvider) -> Option<&str> {
        match provider {
            MetadataProvider::Dlsite => self.dlsite.as_deref(),
            MetadataProvider::Vndb => self.vndb.as_deref(),
            MetadataProvider::Steam => self.steam.as_deref(),
        }
    }
}
