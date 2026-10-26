use crate::domain::keyword_rule::KeywordRule;
use crate::domain::work_type_rule::WorkTypeRule;

/// Every rule the suggestions use.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClassificationRules {
    work_types: Vec<WorkTypeRule>,
    keywords: Vec<KeywordRule>,
}

impl ClassificationRules {
    pub(crate) fn new(work_types: Vec<WorkTypeRule>, keywords: Vec<KeywordRule>) -> Self {
        Self {
            work_types,
            keywords,
        }
    }

    pub(crate) fn work_types(&self) -> &[WorkTypeRule] {
        &self.work_types
    }

    pub(crate) fn keywords(&self) -> &[KeywordRule] {
        &self.keywords
    }
}
