use crate::domain::game_work::WorkDetail;

/// A work's page on one site, normalised across sites. Fields the site does not have stay empty.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ProductDetail {
    work_name: Option<String>,
    maker_name: Option<String>,
    detail: WorkDetail,
}

impl ProductDetail {
    pub(crate) fn new(
        work_name: Option<String>,
        maker_name: Option<String>,
        detail: WorkDetail,
    ) -> Self {
        Self {
            work_name,
            maker_name,
            detail,
        }
    }

    pub(crate) fn work_name(&self) -> Option<&str> {
        self.work_name.as_deref()
    }

    pub(crate) fn maker_name(&self) -> Option<&str> {
        self.maker_name.as_deref()
    }

    pub(crate) fn detail(&self) -> &WorkDetail {
        &self.detail
    }

    pub(crate) fn into_parts(self) -> (Option<String>, Option<String>, WorkDetail) {
        (self.work_name, self.maker_name, self.detail)
    }
}
