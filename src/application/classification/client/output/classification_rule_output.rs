use serde::Serialize;

use crate::domain::keyword_rule::{KeywordRule, KeywordRuleId};
use crate::domain::metadata_source::MetadataProvider;
use crate::domain::work_type_rule::WorkTypeRule;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListClassificationRulesOutput {
    pub(crate) work_type_rules: Vec<WorkTypeRuleOutput>,
    pub(crate) keyword_rules: Vec<KeywordRuleOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkTypeRuleOutput {
    source: &'static str,
    work_type: String,
    code: Option<String>,
    confident: bool,
}

impl From<&WorkTypeRule> for WorkTypeRuleOutput {
    fn from(rule: &WorkTypeRule) -> Self {
        Self {
            source: rule.source().name(),
            work_type: rule.work_type().to_owned(),
            code: rule.code().map(str::to_owned),
            confident: rule.confident(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeywordRuleOutput {
    id: KeywordRuleId,
    source: Option<&'static str>,
    keyword: String,
    code: String,
    contests_type: bool,
}

impl From<&KeywordRule> for KeywordRuleOutput {
    fn from(rule: &KeywordRule) -> Self {
        Self {
            id: rule.id(),
            source: rule.source().map(MetadataProvider::name),
            keyword: rule.keyword().to_owned(),
            code: rule.code().to_owned(),
            contests_type: rule.contests_type(),
        }
    }
}
