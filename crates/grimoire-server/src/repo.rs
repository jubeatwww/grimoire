//! Database helpers shared by several route modules.

use crate::sources::Src;
use grimoire_app::{
    classification::{KeywordRule, Rules, SourceFacts, WorkFacts, WorkTypeRule},
    metadata_source::ProductDetail,
    vocabulary::{CategoryDef, CodeDef, Vocabulary},
};
use sqlx::{PgExecutor, PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn load_vocabulary(db: &PgPool) -> Result<Vocabulary, sqlx::Error> {
    let categories = sqlx::query(
        "SELECT id, label, folder_name, sort_order FROM categories ORDER BY sort_order, id",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| CategoryDef {
        id: row.get("id"),
        label: row.get("label"),
        folder_name: row.get("folder_name"),
        sort_order: row.get("sort_order"),
    })
    .collect();

    let codes = sqlx::query(
        "SELECT code, label, category_id, family, accepts_prefix, aliases, sort_order
           FROM genre_codes
          ORDER BY sort_order, code",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| {
        let aliases: serde_json::Value = row.get("aliases");
        CodeDef {
            code: row.get("code"),
            label: row.get("label"),
            category_id: row.get("category_id"),
            family: row.get("family"),
            accepts_prefix: row.get("accepts_prefix"),
            aliases: serde_json::from_value(aliases).unwrap_or_default(),
            sort_order: row.get("sort_order"),
        }
    })
    .collect();

    Ok(Vocabulary::new(categories, codes))
}

/// Replace one source's raw tags and work type for a work.
pub async fn upsert_source_data(
    db: impl PgExecutor<'_>,
    game_work_id: Uuid,
    src: Src,
    detail: &ProductDetail,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO work_source_data (game_work_id, source, tags, work_type, work_type_label, fetched_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (game_work_id, source) DO UPDATE SET
            tags            = EXCLUDED.tags,
            work_type       = EXCLUDED.work_type,
            work_type_label = EXCLUDED.work_type_label,
            fetched_at      = now()",
    )
    .bind(game_work_id)
    .bind(src.name())
    .bind(serde_json::json!(detail.tags))
    .bind(&detail.work_type)
    .bind(&detail.work_type_label)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn load_rules(db: &PgPool) -> Result<Rules, sqlx::Error> {
    let work_types = sqlx::query(
        "SELECT source, work_type, code, confident FROM work_type_rules ORDER BY source, work_type",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| WorkTypeRule {
        source: row.get("source"),
        work_type: row.get("work_type"),
        code: row.get("code"),
        confident: row.get("confident"),
    })
    .collect();
    let keywords = sqlx::query(
        "SELECT id, source, keyword, code, contests_type
           FROM keyword_rules
          ORDER BY created_at, keyword",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|row| KeywordRule {
        id: row.get::<Uuid, _>("id").to_string(),
        source: row.get("source"),
        keyword: row.get("keyword"),
        code: row.get("code"),
        contests_type: row.get("contests_type"),
    })
    .collect();
    Ok(Rules {
        work_types,
        keywords,
    })
}

/// Facts for every work that still needs a category: unconfirmed works with at
/// least one present, non-excluded file.
pub async fn load_unclassified_facts(db: &PgPool) -> Result<Vec<(Uuid, WorkFacts)>, sqlx::Error> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT g.id
           FROM game_works g
          WHERE g.category_confirmed_at IS NULL
            AND EXISTS (SELECT 1 FROM inventory_items i
                         WHERE i.game_work_id = g.id
                           AND NOT i.missing
                           AND i.organization_status <> 'ignored')
          ORDER BY g.display_title",
    )
    .fetch_all(db)
    .await?;

    let mut facts: HashMap<Uuid, WorkFacts> =
        ids.iter().map(|id| (*id, WorkFacts::default())).collect();

    for row in sqlx::query(
        "SELECT game_work_id, source, tags, work_type
           FROM work_source_data
          WHERE game_work_id = ANY($1)
          ORDER BY source",
    )
    .bind(&ids)
    .fetch_all(db)
    .await?
    {
        let id: Uuid = row.get("game_work_id");
        let tags: serde_json::Value = row.get("tags");
        if let Some(f) = facts.get_mut(&id) {
            f.sources.push(SourceFacts {
                source: row.get("source"),
                work_type: row.get("work_type"),
                tags: serde_json::from_value(tags).unwrap_or_default(),
            });
        }
    }

    for row in sqlx::query(
        "SELECT game_work_id, legacy_location, primary_category
           FROM inventory_items
          WHERE game_work_id = ANY($1) AND NOT missing",
    )
    .bind(&ids)
    .fetch_all(db)
    .await?
    {
        let id: Uuid = row.get("game_work_id");
        if let Some(f) = facts.get_mut(&id) {
            if let Some(folder) = row.get::<Option<String>, _>("legacy_location") {
                f.legacy_locations.push(folder);
            }
            if let Some(old) = row.get::<Option<String>, _>("primary_category") {
                f.old_primaries.push(old);
            }
        }
    }

    Ok(ids
        .into_iter()
        .map(|id| {
            let f = facts.remove(&id).unwrap_or_default();
            (id, f)
        })
        .collect())
}
