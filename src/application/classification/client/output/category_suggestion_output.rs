use serde::Serialize;

use crate::domain::category_suggestion::CategorySuggestion;
use crate::domain::game_work::GameWorkId;

#[derive(Debug, Serialize)]
pub(crate) struct ListCategorySuggestionsOutput {
    pub(crate) suggestions: Vec<CategorySuggestionOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CategorySuggestionOutput {
    game_work_id: GameWorkId,
    category: Option<String>,
    codes: Vec<String>,
    confident: bool,
    reasons: Vec<String>,
}

impl From<(GameWorkId, &CategorySuggestion)> for CategorySuggestionOutput {
    fn from((game_work_id, suggestion): (GameWorkId, &CategorySuggestion)) -> Self {
        Self {
            game_work_id,
            category: suggestion.category().map(str::to_owned),
            codes: suggestion.codes().to_vec(),
            confident: suggestion.confident(),
            reasons: suggestion.reasons().to_vec(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ConfirmClassificationsOutput {
    pub(crate) updated: usize,
}
