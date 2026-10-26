use chrono::NaiveDate;

use super::DetailPrecedence;

/// What a work's page says: description, tags, images, sales figures and the site's own work
/// type. Built by the metadata sources; kept on the work.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct WorkDetail {
    description: Option<String>,
    release_date: Option<NaiveDate>,
    series: Option<String>,
    tags: Vec<String>,
    cover_image_url: Option<String>,
    preview_image_urls: Vec<String>,
    file_type: Option<String>,
    file_size_bytes: Option<i64>,
    dl_count: Option<i32>,
    rate_average: Option<f32>,
    rate_count: Option<i32>,
    price_jpy: Option<i32>,
    work_type: Option<String>,
    work_type_label: Option<String>,
}

impl WorkDetail {
    pub(crate) fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    pub(crate) fn with_release_date(mut self, release_date: Option<NaiveDate>) -> Self {
        self.release_date = release_date;
        self
    }

    pub(crate) fn with_series(mut self, series: Option<String>) -> Self {
        self.series = series;
        self
    }

    pub(crate) fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub(crate) fn with_cover_image_url(mut self, cover_image_url: Option<String>) -> Self {
        self.cover_image_url = cover_image_url;
        self
    }

    pub(crate) fn with_preview_image_urls(mut self, preview_image_urls: Vec<String>) -> Self {
        self.preview_image_urls = preview_image_urls;
        self
    }

    pub(crate) fn with_file(
        mut self,
        file_type: Option<String>,
        file_size_bytes: Option<i64>,
    ) -> Self {
        self.file_type = file_type;
        self.file_size_bytes = file_size_bytes;
        self
    }

    pub(crate) fn with_sales(
        mut self,
        dl_count: Option<i32>,
        rate_average: Option<f32>,
        rate_count: Option<i32>,
        price_jpy: Option<i32>,
    ) -> Self {
        self.dl_count = dl_count;
        self.rate_average = rate_average;
        self.rate_count = rate_count;
        self.price_jpy = price_jpy;
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

    /// `self` is what the work has; `incoming` is a source's page. Tags and previews always
    /// follow the page; release date and cover are never blanked by it; the rest is replaced
    /// outright by `Overwrite` and only where the page has a value by `Merge`.
    pub(crate) fn merged(self, incoming: WorkDetail, precedence: DetailPrecedence) -> Self {
        let overwrite = precedence == DetailPrecedence::Overwrite;
        let pick = |new: Option<String>, old: Option<String>| {
            if overwrite { new } else { new.or(old) }
        };
        Self {
            description: pick(incoming.description, self.description),
            release_date: incoming.release_date.or(self.release_date),
            series: pick(incoming.series, self.series),
            tags: incoming.tags,
            cover_image_url: incoming.cover_image_url.or(self.cover_image_url),
            preview_image_urls: incoming.preview_image_urls,
            file_type: pick(incoming.file_type, self.file_type),
            file_size_bytes: if overwrite {
                incoming.file_size_bytes
            } else {
                incoming.file_size_bytes.or(self.file_size_bytes)
            },
            dl_count: if overwrite {
                incoming.dl_count
            } else {
                incoming.dl_count.or(self.dl_count)
            },
            rate_average: if overwrite {
                incoming.rate_average
            } else {
                incoming.rate_average.or(self.rate_average)
            },
            rate_count: if overwrite {
                incoming.rate_count
            } else {
                incoming.rate_count.or(self.rate_count)
            },
            price_jpy: if overwrite {
                incoming.price_jpy
            } else {
                incoming.price_jpy.or(self.price_jpy)
            },
            work_type: pick(incoming.work_type, self.work_type),
            work_type_label: pick(incoming.work_type_label, self.work_type_label),
        }
    }

    pub(crate) fn with_cover_if_missing(mut self, cover_image_url: String) -> Self {
        self.cover_image_url = self.cover_image_url.or(Some(cover_image_url));
        self
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn release_date(&self) -> Option<NaiveDate> {
        self.release_date
    }

    pub(crate) fn series(&self) -> Option<&str> {
        self.series.as_deref()
    }

    pub(crate) fn tags(&self) -> &[String] {
        &self.tags
    }

    pub(crate) fn cover_image_url(&self) -> Option<&str> {
        self.cover_image_url.as_deref()
    }

    pub(crate) fn preview_image_urls(&self) -> &[String] {
        &self.preview_image_urls
    }

    pub(crate) fn file_type(&self) -> Option<&str> {
        self.file_type.as_deref()
    }

    pub(crate) fn file_size_bytes(&self) -> Option<i64> {
        self.file_size_bytes
    }

    pub(crate) fn dl_count(&self) -> Option<i32> {
        self.dl_count
    }

    pub(crate) fn rate_average(&self) -> Option<f32> {
        self.rate_average
    }

    pub(crate) fn rate_count(&self) -> Option<i32> {
        self.rate_count
    }

    pub(crate) fn price_jpy(&self) -> Option<i32> {
        self.price_jpy
    }

    pub(crate) fn work_type(&self) -> Option<&str> {
        self.work_type.as_deref()
    }

    pub(crate) fn work_type_label(&self) -> Option<&str> {
        self.work_type_label.as_deref()
    }
}
