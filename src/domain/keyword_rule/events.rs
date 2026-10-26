use serde::Serialize;

use super::KeywordRuleId;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct KeywordRuleDefined {
    pub(crate) rule_id: KeywordRuleId,
    pub(crate) keyword: String,
}
