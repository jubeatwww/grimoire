//! Primary-category suggestions for works that have not been classified yet.
//!
//! A pure derivation over a work's facts and the editable rules; see
//! `docs/library-reclassification.md` ("Suggestions") for the agreed policy:
//!
//! 1. A source's work type rule proposes a code and its category.
//! 2. Keyword rules over that source's tags add codes. A keyword code in the work type's own
//!    category replaces the type code (FTG refines ACT). A keyword flagged `contests_type`
//!    landing in another category makes the suggestion not confident (アクションRPG on a DLsite
//!    RPG).
//! 3. Without a work type category, keyword codes decide, then hints. Such suggestions are never
//!    confident.
//! 4. Confident needs a confident type rule, agreeing sources, no contesting keyword, and every
//!    hint containing the suggested category.

mod classification_rules;
mod derivation;
mod hints;
#[cfg(test)]
mod tests;
mod work_facts;

pub(crate) use classification_rules::ClassificationRules;
pub(crate) use work_facts::{SourceFacts, WorkFacts};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategorySuggestion {
    category: Option<String>,
    codes: Vec<String>,
    confident: bool,
    reasons: Vec<String>,
}

impl CategorySuggestion {
    pub(crate) fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }

    pub(crate) fn codes(&self) -> &[String] {
        &self.codes
    }

    pub(crate) fn confident(&self) -> bool {
        self.confident
    }

    pub(crate) fn reasons(&self) -> &[String] {
        &self.reasons
    }
}
