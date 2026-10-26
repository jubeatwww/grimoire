/// What a work is called and who made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkIdentity {
    canonical_title: String,
    display_title: String,
    circle: Option<String>,
}

impl WorkIdentity {
    pub(crate) fn new(
        canonical_title: String,
        display_title: String,
        circle: Option<String>,
    ) -> Self {
        Self {
            canonical_title,
            display_title,
            circle,
        }
    }

    /// A source's name for the work and its maker, where the source has one.
    pub(crate) fn renamed(&self, title: Option<String>, circle: Option<String>) -> Self {
        Self {
            canonical_title: title
                .clone()
                .unwrap_or_else(|| self.canonical_title.clone()),
            display_title: title.unwrap_or_else(|| self.display_title.clone()),
            circle: circle.or_else(|| self.circle.clone()),
        }
    }

    pub(crate) fn with_display_title(&self, display_title: String) -> Self {
        Self {
            display_title,
            ..self.clone()
        }
    }

    pub(crate) fn canonical_title(&self) -> &str {
        &self.canonical_title
    }

    pub(crate) fn display_title(&self) -> &str {
        &self.display_title
    }

    pub(crate) fn circle(&self) -> Option<&str> {
        self.circle.as_deref()
    }
}
