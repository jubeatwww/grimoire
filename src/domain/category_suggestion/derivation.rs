use super::hints::{legacy_folder_hint, old_primary_hint};
use super::{CategorySuggestion, ClassificationRules, SourceFacts, WorkFacts};
use crate::domain::keyword_rule::KeywordRule;
use crate::domain::vocabulary::Vocabulary;

struct KeywordHit<'a> {
    rule: &'a KeywordRule,
    code: String,
    category: String,
    tag: &'a str,
}

fn keyword_hits<'a>(
    vocab: &Vocabulary,
    rules: &'a ClassificationRules,
    source: &'a SourceFacts,
) -> Vec<KeywordHit<'a>> {
    let mut hits: Vec<KeywordHit<'a>> = Vec::new();
    for rule in rules.keywords() {
        if !rule.applies_to(source.provider()) {
            continue;
        }
        let Some(tag) = source.tags().iter().find(|t| rule.matches_tag(t)) else {
            continue;
        };
        let Some(resolved) = vocab.resolve(rule.code()) else {
            continue;
        };
        let code = resolved.canonical();
        if hits.iter().any(|h| h.code == code) {
            continue;
        }
        hits.push(KeywordHit {
            rule,
            code,
            category: resolved.base().category_id().to_owned(),
            tag,
        });
    }
    hits
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !list.contains(&value) {
        list.push(value);
    }
}

fn category_name(vocab: &Vocabulary, id: &str) -> String {
    vocab
        .category(id)
        .map(|c| c.folder_name().to_owned())
        .unwrap_or_else(|| id.to_string())
}

/// The category with the most keyword votes; ties go to the lower folder number.
fn most_voted(vocab: &Vocabulary, votes: &[String]) -> Option<String> {
    let mut best: Option<(&String, usize, i32)> = None;
    for candidate in votes {
        let count = votes.iter().filter(|v| *v == candidate).count();
        let order = vocab
            .category(candidate)
            .map(|c| c.sort_order())
            .unwrap_or(i32::MAX);
        let better = match best {
            None => true,
            Some((_, best_count, best_order)) => {
                count > best_count || (count == best_count && order < best_order)
            }
        };
        if better {
            best = Some((candidate, count, order));
        }
    }
    best.map(|(c, _, _)| c.clone())
}

impl CategorySuggestion {
    /// The suggestion for one work.
    pub(crate) fn derive(
        vocab: &Vocabulary,
        rules: &ClassificationRules,
        facts: &WorkFacts,
    ) -> Self {
        let mut codes: Vec<String> = Vec::new();
        let mut reasons: Vec<String> = Vec::new();
        let mut contested = false;
        // (category, rule is confident) per source that had a usable type rule.
        let mut type_votes: Vec<(String, bool)> = Vec::new();
        // Keyword categories from sources whose type rule deferred to tags.
        let mut tag_votes: Vec<String> = Vec::new();

        for source in facts.sources() {
            let source_name = source.provider().name();
            let hits = keyword_hits(vocab, rules, source);
            let rule = source.work_type().and_then(|wt| {
                rules.work_types().iter().find(|r| {
                    r.source() == source.provider() && r.work_type().eq_ignore_ascii_case(wt)
                })
            });
            let typed = rule.and_then(|r| {
                let resolved = vocab.resolve(r.code()?)?;
                Some((
                    r,
                    resolved.canonical(),
                    resolved.base().category_id().to_owned(),
                ))
            });

            match typed {
                Some((rule, type_code, category)) => {
                    let refinements: Vec<&KeywordHit> =
                        hits.iter().filter(|h| h.category == category).collect();
                    if refinements.is_empty() {
                        push_unique(&mut codes, type_code.clone());
                        reasons.push(format!(
                            "{} {} → {}",
                            source_name,
                            rule.work_type(),
                            type_code
                        ));
                    } else {
                        for hit in &refinements {
                            push_unique(&mut codes, hit.code.clone());
                            reasons.push(format!(
                                "{} {} refined by tag {} → {}",
                                source_name,
                                rule.work_type(),
                                hit.tag,
                                hit.code
                            ));
                        }
                    }
                    for hit in hits.iter().filter(|h| h.category != category) {
                        push_unique(&mut codes, hit.code.clone());
                        if hit.rule.contests_type() {
                            contested = true;
                            reasons.push(format!(
                                "tag {} suggests {} instead of {}",
                                hit.tag,
                                category_name(vocab, &hit.category),
                                category_name(vocab, &category)
                            ));
                        } else {
                            reasons.push(format!("tag {} adds {}", hit.tag, hit.code));
                        }
                    }
                    type_votes.push((category, rule.confident()));
                }
                None => {
                    match (source.work_type(), rule) {
                        (Some(wt), None) => {
                            reasons.push(format!("no rule for {} type {}", source_name, wt))
                        }
                        (Some(wt), Some(_)) => {
                            reasons.push(format!("{} {} → decided by tags", source_name, wt))
                        }
                        (None, _) => {}
                    }
                    for hit in &hits {
                        push_unique(&mut codes, hit.code.clone());
                        tag_votes.push(hit.category.clone());
                        reasons.push(format!("tag {} → {}", hit.tag, hit.code));
                    }
                }
            }
        }

        let mut type_categories: Vec<String> = Vec::new();
        for (category, _) in &type_votes {
            push_unique(&mut type_categories, category.clone());
        }

        let mut from_types = false;
        let mut category: Option<String> = match type_categories.as_slice() {
            [] => None,
            [only] => {
                from_types = true;
                Some(only.clone())
            }
            [first, ..] => {
                from_types = true;
                contested = true;
                reasons.push(format!(
                    "sources disagree: {}",
                    type_categories
                        .iter()
                        .map(|c| category_name(vocab, c))
                        .collect::<Vec<_>>()
                        .join(" / ")
                ));
                Some(first.clone())
            }
        };

        if category.is_none() {
            category = most_voted(vocab, &tag_votes);
            if let Some(c) = &category {
                let distinct = {
                    let mut d: Vec<&String> = tag_votes.iter().collect();
                    d.sort();
                    d.dedup();
                    d.len()
                };
                if distinct > 1 {
                    reasons.push(format!(
                        "tags point to several categories; picked {}",
                        category_name(vocab, c)
                    ));
                }
            }
        }

        // Hints: non-empty hint sets per file and per old category value.
        let mut hint_sets: Vec<(String, Vec<String>)> = Vec::new();
        for folder in facts.legacy_locations() {
            let set = legacy_folder_hint(vocab, folder);
            if !set.is_empty() {
                hint_sets.push((format!("legacy folder {folder}"), set));
            }
        }
        for old in facts.old_primaries() {
            let set = old_primary_hint(old);
            if !set.is_empty() {
                hint_sets.push((format!("old category {old}"), set));
            }
        }

        match &category {
            Some(c) => {
                for (what, set) in &hint_sets {
                    if !set.contains(c) {
                        contested = true;
                        reasons.push(format!(
                            "{what} points to {}",
                            set.iter()
                                .map(|s| category_name(vocab, s))
                                .collect::<Vec<_>>()
                                .join(" or ")
                        ));
                    }
                }
            }
            None => {
                // Only hints left: take the categories every hint agrees on.
                let mut common: Option<Vec<String>> = None;
                for (_, set) in &hint_sets {
                    common = Some(match common {
                        None => set.clone(),
                        Some(prev) => prev.into_iter().filter(|c| set.contains(c)).collect(),
                    });
                }
                match common.as_deref() {
                    Some([only]) => {
                        category = Some(only.clone());
                        reasons.push(format!(
                            "only hints available: {}",
                            hint_sets
                                .iter()
                                .map(|(w, _)| w.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    Some(several) if several.len() > 1 => reasons.push(format!(
                        "hints allow {}",
                        several
                            .iter()
                            .map(|s| category_name(vocab, s))
                            .collect::<Vec<_>>()
                            .join(" or ")
                    )),
                    Some(_) => reasons.push("hints disagree".to_string()),
                    None => {}
                }
            }
        }

        if category.is_none() && reasons.is_empty() {
            reasons.push("no source data or hints".to_string());
        }

        let confident = category.is_some()
            && from_types
            && type_votes.iter().all(|(_, confident)| *confident)
            && !contested;

        // Final code list: known codes only, plus the category's own code.
        let known: Vec<&str> = codes
            .iter()
            .map(String::as_str)
            .filter(|c| vocab.resolve(c).is_some())
            .collect();
        let codes = vocab
            .prepare_classification(category.as_deref(), known)
            .unwrap_or_default();

        Self {
            category,
            codes,
            confident,
            reasons,
        }
    }
}
