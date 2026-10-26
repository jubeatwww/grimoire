use super::WorkDetail;

/// A manual edit of a work. `None` leaves a field alone. For the work type, its label and the
/// cover an empty string clears the field, so the user can strip an over-eager source value. A
/// blank title keeps the old one. A preview list, even an empty one, replaces the whole set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkEdit {
    display_title: Option<String>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    cover_image_url: Option<String>,
    preview_image_urls: Option<Vec<String>>,
}

impl WorkEdit {
    pub(crate) fn new(
        display_title: Option<String>,
        work_type: Option<String>,
        work_type_label: Option<String>,
        cover_image_url: Option<String>,
        preview_image_urls: Option<Vec<String>>,
    ) -> Self {
        Self {
            display_title,
            work_type,
            work_type_label,
            cover_image_url,
            preview_image_urls,
        }
    }

    /// The edited display title and detail.
    pub(crate) fn apply_to(self, display_title: &str, detail: WorkDetail) -> (String, WorkDetail) {
        let title = self
            .display_title
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| display_title.to_owned());
        let cleared = |edit: Option<String>, old: Option<&str>| match edit {
            None => old.map(str::to_owned),
            Some(v) if v.is_empty() => None,
            Some(v) => Some(v),
        };
        let work_type = cleared(self.work_type, detail.work_type());
        let work_type_label = cleared(self.work_type_label, detail.work_type_label());
        let cover = cleared(self.cover_image_url, detail.cover_image_url());
        let previews = self
            .preview_image_urls
            .unwrap_or_else(|| detail.preview_image_urls().to_vec());
        let detail = detail
            .with_work_type(work_type, work_type_label)
            .with_cover_image_url(cover)
            .with_preview_image_urls(previews);
        (title, detail)
    }
}
