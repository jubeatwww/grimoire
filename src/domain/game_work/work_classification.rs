use crate::kernel::Timestamp;

/// The work's primary category (its NAS folder) and its genre codes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkClassification {
    primary_category: Option<String>,
    genre_facets: Vec<String>,
    confirmed_at: Option<Timestamp>,
}

impl WorkClassification {
    pub(crate) fn unclassified() -> Self {
        Self::default()
    }

    pub(crate) fn new(
        primary_category: Option<String>,
        genre_facets: Vec<String>,
        confirmed_at: Option<Timestamp>,
    ) -> Self {
        Self {
            primary_category,
            genre_facets,
            confirmed_at,
        }
    }

    pub(crate) fn primary_category(&self) -> Option<&str> {
        self.primary_category.as_deref()
    }

    pub(crate) fn genre_facets(&self) -> &[String] {
        &self.genre_facets
    }

    pub(crate) fn confirmed_at(&self) -> Option<Timestamp> {
        self.confirmed_at
    }
}
