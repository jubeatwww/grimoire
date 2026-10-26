use serde::Serialize;

use crate::domain::genre_code::GenreCode;
use crate::domain::vocabulary::{Category, Vocabulary};

#[derive(Debug, Serialize)]
pub(crate) struct VocabularyOutput {
    categories: Vec<CategoryOutput>,
    codes: Vec<GenreCodeOutput>,
}

impl From<&Vocabulary> for VocabularyOutput {
    fn from(vocabulary: &Vocabulary) -> Self {
        Self {
            categories: vocabulary
                .categories()
                .iter()
                .map(CategoryOutput::from)
                .collect(),
            codes: vocabulary
                .codes()
                .iter()
                .map(GenreCodeOutput::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CategoryOutput {
    id: String,
    label: String,
    folder_name: String,
    sort_order: i32,
}

impl From<&Category> for CategoryOutput {
    fn from(category: &Category) -> Self {
        Self {
            id: category.id().to_owned(),
            label: category.label().to_owned(),
            folder_name: category.folder_name().to_owned(),
            sort_order: category.sort_order(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenreCodeOutput {
    code: String,
    label: String,
    category_id: String,
    family: String,
    accepts_prefix: bool,
    aliases: Vec<String>,
    sort_order: i32,
}

impl From<&GenreCode> for GenreCodeOutput {
    fn from(code: &GenreCode) -> Self {
        Self {
            code: code.code().to_owned(),
            label: code.label().to_owned(),
            category_id: code.category_id().to_owned(),
            family: code.family().to_owned(),
            accepts_prefix: code.accepts_prefix(),
            aliases: code.aliases().to_vec(),
            sort_order: code.sort_order(),
        }
    }
}
