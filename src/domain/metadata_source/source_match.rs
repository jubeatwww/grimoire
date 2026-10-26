/// One search hit on a site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceMatch {
    source_work_id: String,
    rank: i32,
    title: String,
    circle: Option<String>,
    cover_url: Option<String>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    intro: Option<String>,
}

impl SourceMatch {
    /// `rank` starts at 1 for the site's best hit.
    pub(crate) fn new(source_work_id: String, rank: i32, title: String) -> Self {
        Self {
            source_work_id,
            rank,
            title,
            circle: None,
            cover_url: None,
            work_type: None,
            work_type_label: None,
            intro: None,
        }
    }

    pub(crate) fn with_circle(mut self, circle: Option<String>) -> Self {
        self.circle = circle;
        self
    }

    pub(crate) fn with_cover_url(mut self, cover_url: Option<String>) -> Self {
        self.cover_url = cover_url;
        self
    }

    pub(crate) fn with_work_type(
        mut self,
        work_type: Option<String>,
        label: Option<String>,
    ) -> Self {
        self.work_type = work_type;
        self.work_type_label = label;
        self
    }

    /// A short blurb shown under the hit.
    pub(crate) fn with_intro(mut self, intro: Option<String>) -> Self {
        self.intro = intro;
        self
    }

    pub(crate) fn source_work_id(&self) -> &str {
        &self.source_work_id
    }

    pub(crate) fn rank(&self) -> i32 {
        self.rank
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn circle(&self) -> Option<&str> {
        self.circle.as_deref()
    }

    pub(crate) fn cover_url(&self) -> Option<&str> {
        self.cover_url.as_deref()
    }

    pub(crate) fn work_type(&self) -> Option<&str> {
        self.work_type.as_deref()
    }

    pub(crate) fn work_type_label(&self) -> Option<&str> {
        self.work_type_label.as_deref()
    }

    pub(crate) fn intro(&self) -> Option<&str> {
        self.intro.as_deref()
    }
}
