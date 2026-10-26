//! A game work: one title, whatever number of files the library holds for it.

mod classification_recorder;
mod detail_precedence;
mod events;
mod external_ids;
mod game_work_factory;
mod game_work_repository;
mod work_classification;
mod work_detail;
mod work_edit;
mod work_identity;

use crate::domain::metadata_source::{MetadataProvider, ProductDetail};
use crate::domain::vocabulary::{CategoryId, Vocabulary, VocabularyError};
use crate::kernel::{Id, Timestamp};

pub(crate) use classification_recorder::{ClassificationRecorder, ClassificationRecorderError};
pub(crate) use detail_precedence::DetailPrecedence;
pub(crate) use events::{
    GameWorkClassified, GameWorkCoverPrefilled, GameWorkEdited, GameWorkEnriched, GameWorkEntered,
    GameWorkRegistered,
};
pub(crate) use external_ids::ExternalIds;
pub(crate) use game_work_factory::{GameWorkFactory, WorkLookup};
pub(crate) use game_work_repository::{GameWorkRepository, GameWorkRepositoryError};
pub(crate) use work_classification::WorkClassification;
pub(crate) use work_detail::WorkDetail;
pub(crate) use work_edit::WorkEdit;
pub(crate) use work_identity::WorkIdentity;

#[cfg(test)]
pub(crate) mod fakes {
    pub(crate) use super::game_work_repository::fakes::FakeGameWorkRepository;
}

pub(crate) type GameWorkId = Id<GameWork>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameWork {
    id: GameWorkId,
    identity: WorkIdentity,
    source_urls: Vec<String>,
    external_ids: ExternalIds,
    detail: WorkDetail,
    enriched_at: Option<Timestamp>,
    classification: WorkClassification,
}

impl GameWork {
    /// A work first seen on a metadata source. Its title and circle come from the source.
    pub(crate) fn register(
        id: GameWorkId,
        provider: MetadataProvider,
        external_id: &str,
        title: &str,
        circle: Option<String>,
    ) -> (Self, GameWorkRegistered) {
        let work = Self {
            id,
            identity: WorkIdentity::new(title.to_owned(), title.to_owned(), circle),
            source_urls: vec![provider.work_url(external_id)],
            external_ids: ExternalIds::none().with(provider, external_id.to_owned()),
            detail: WorkDetail::default(),
            enriched_at: None,
            classification: WorkClassification::unclassified(),
        };
        let registered = GameWorkRegistered {
            work_id: id,
            provider,
            external_id: external_id.to_owned(),
        };
        (work, registered)
    }

    /// A work the user fills in by hand for a file no source knows, titled after the file. It
    /// counts as enriched so it does not land in the missing-detail queue.
    pub(crate) fn enter_manually(
        id: GameWorkId,
        title: &str,
        at: Timestamp,
    ) -> (Self, GameWorkEntered) {
        let work = Self {
            id,
            identity: WorkIdentity::new(title.to_owned(), title.to_owned(), None),
            source_urls: Vec::new(),
            external_ids: ExternalIds::none(),
            detail: WorkDetail::default(),
            enriched_at: Some(at),
            classification: WorkClassification::unclassified(),
        };
        (work, GameWorkEntered { work_id: id, at })
    }

    /// Takes a source's page into the work. With `Overwrite` the source also renames the work
    /// and its circle, so a locale-switched refresh really updates the display string.
    pub(crate) fn apply_detail(
        &mut self,
        product: ProductDetail,
        precedence: DetailPrecedence,
        at: Timestamp,
    ) -> GameWorkEnriched {
        let (work_name, maker_name, detail) = product.into_parts();
        if precedence == DetailPrecedence::Overwrite {
            self.identity = self.identity.renamed(work_name, maker_name);
        }
        self.detail = self.detail.clone().merged(detail, precedence);
        self.enriched_at = Some(at);
        GameWorkEnriched {
            work_id: self.id,
            at,
        }
    }

    /// Uses a search hit's cover while the work has none yet.
    pub(crate) fn prefill_cover(&mut self, cover_url: String) -> GameWorkCoverPrefilled {
        self.detail = self.detail.clone().with_cover_if_missing(cover_url);
        GameWorkCoverPrefilled { work_id: self.id }
    }

    /// A manual edit from the item panel.
    pub(crate) fn edit(&mut self, edit: WorkEdit) -> GameWorkEdited {
        let (title, detail) = edit.apply_to(self.identity.display_title(), self.detail.clone());
        self.identity = self.identity.with_display_title(title);
        self.detail = detail;
        GameWorkEdited { work_id: self.id }
    }

    /// Sets the primary category and genre codes. Codes are stored canonical and deduplicated,
    /// and a category always comes with one of its own codes (see
    /// `Vocabulary::prepare_classification`). A category confirms the classification; no
    /// category clears it.
    pub(crate) fn classify(
        &mut self,
        vocabulary: &Vocabulary,
        category: Option<&CategoryId>,
        genre_facets: &[String],
        at: Timestamp,
    ) -> Result<GameWorkClassified, VocabularyError> {
        let category = category.map(|c| c.as_str().to_owned());
        let facets = vocabulary
            .prepare_classification(category.as_deref(), genre_facets.iter().map(String::as_str))?;
        let confirmed_at = category.as_ref().map(|_| at);
        self.classification = WorkClassification::new(category.clone(), facets, confirmed_at);
        Ok(GameWorkClassified {
            work_id: self.id,
            category,
            at,
        })
    }

    /// Rebuilds a stored work. Repositories only.
    pub(crate) fn from_persisted(
        id: GameWorkId,
        identity: WorkIdentity,
        source_urls: Vec<String>,
        external_ids: ExternalIds,
        detail: WorkDetail,
        enriched_at: Option<Timestamp>,
        classification: WorkClassification,
    ) -> Self {
        Self {
            id,
            identity,
            source_urls,
            external_ids,
            detail,
            enriched_at,
            classification,
        }
    }

    pub(crate) fn id(&self) -> GameWorkId {
        self.id
    }

    pub(crate) fn identity(&self) -> &WorkIdentity {
        &self.identity
    }

    pub(crate) fn source_urls(&self) -> &[String] {
        &self.source_urls
    }

    pub(crate) fn external_ids(&self) -> &ExternalIds {
        &self.external_ids
    }

    pub(crate) fn detail(&self) -> &WorkDetail {
        &self.detail
    }

    pub(crate) fn enriched_at(&self) -> Option<Timestamp> {
        self.enriched_at
    }

    pub(crate) fn classification(&self) -> &WorkClassification {
        &self.classification
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn at(day: u32) -> Timestamp {
        Timestamp::from_utc(Utc.with_ymd_and_hms(2026, 1, day, 0, 0, 0).unwrap())
    }

    fn work() -> GameWork {
        let (mut work, _registered) = GameWork::register(
            GameWorkId::from_raw(uuid::Uuid::from_u128(1)),
            MetadataProvider::Dlsite,
            "RJ01234567",
            "Old Title",
            Some("Old Circle".into()),
        );
        let detail = WorkDetail::default()
            .with_description(Some("old description".into()))
            .with_series(Some("old series".into()))
            .with_cover_image_url(Some("old-cover".into()))
            .with_work_type(Some("RPG".into()), Some("ロールプレイング".into()));
        let _enriched = work.apply_detail(
            ProductDetail::new(None, None, detail),
            DetailPrecedence::Merge,
            at(1),
        );
        work
    }

    #[test]
    fn registering_links_the_source_and_its_page() {
        let work = work();
        assert_eq!(
            work.external_ids().get(MetadataProvider::Dlsite),
            Some("RJ01234567")
        );
        assert_eq!(
            work.source_urls(),
            ["https://www.dlsite.com/maniax/work/=/product_id/RJ01234567.html"]
        );
        assert_eq!(work.identity().display_title(), "Old Title");
    }

    #[test]
    fn overwrite_renames_and_replaces_source_fields() {
        let mut work = work();
        let fresh = ProductDetail::new(
            Some("New Title".into()),
            None,
            WorkDetail::default().with_tags(vec!["RPG".into()]),
        );
        let _enriched = work.apply_detail(fresh, DetailPrecedence::Overwrite, at(2));
        assert_eq!(work.identity().display_title(), "New Title");
        assert_eq!(work.identity().canonical_title(), "New Title");
        assert_eq!(work.identity().circle(), Some("Old Circle"));
        assert_eq!(
            work.detail().description(),
            None,
            "the source has none, so it is cleared"
        );
        assert_eq!(work.detail().series(), None);
        assert_eq!(
            work.detail().cover_image_url(),
            Some("old-cover"),
            "kept when the source has none"
        );
        assert_eq!(work.detail().work_type(), None);
        assert_eq!(work.enriched_at(), Some(at(2)));
    }

    #[test]
    fn merge_keeps_the_work_s_values_where_the_page_has_none() {
        let mut work = work();
        let other = ProductDetail::new(
            Some("Other Title".into()),
            Some("Other Circle".into()),
            WorkDetail::default()
                .with_description(Some("other".into()))
                .with_tags(vec!["ADV".into()])
                .with_preview_image_urls(vec!["p1".into()]),
        );
        let _enriched = work.apply_detail(other, DetailPrecedence::Merge, at(3));
        assert_eq!(work.identity().display_title(), "Old Title");
        assert_eq!(work.identity().circle(), Some("Old Circle"));
        assert_eq!(
            work.detail().description(),
            Some("other"),
            "a value on the page still wins"
        );
        assert_eq!(
            work.detail().series(),
            Some("old series"),
            "a gap on the page keeps the work's"
        );
        assert_eq!(
            work.detail().tags(),
            ["ADV"],
            "tags always follow the latest source"
        );
        assert_eq!(work.detail().preview_image_urls(), ["p1"]);
        assert_eq!(work.detail().work_type(), Some("RPG"));
    }

    #[test]
    fn edit_clears_with_an_empty_string_and_keeps_a_blank_title() {
        let mut work = work();
        let _edited = work.edit(WorkEdit::new(
            Some(String::new()),
            Some(String::new()),
            None,
            Some("new-cover".into()),
            Some(Vec::new()),
        ));
        assert_eq!(work.identity().display_title(), "Old Title");
        assert_eq!(work.detail().work_type(), None);
        assert_eq!(work.detail().work_type_label(), Some("ロールプレイング"));
        assert_eq!(work.detail().cover_image_url(), Some("new-cover"));
    }

    #[test]
    fn classifying_without_a_category_clears_the_confirmation() {
        let vocabulary = crate::domain::vocabulary::tests::sample();
        let rpg = CategoryId::parse("RPG");
        let mut work = work();
        let _classified = work
            .classify(&vocabulary, rpg.as_ref(), &["edu".into()], at(4))
            .unwrap();
        assert_eq!(work.classification().confirmed_at(), Some(at(4)));
        assert_eq!(work.classification().genre_facets(), ["RPG", "EDU"]);
        let _cleared = work
            .classify(&vocabulary, None, &["edu".into()], at(5))
            .unwrap();
        assert_eq!(work.classification().confirmed_at(), None);
        assert_eq!(work.classification().genre_facets(), ["EDU"]);
        assert!(
            work.classify(&vocabulary, rpg.as_ref(), &["nope".into()], at(6))
                .is_err()
        );
    }

    #[test]
    fn prefilled_cover_never_replaces_one() {
        let mut work = work();
        let _prefilled = work.prefill_cover("candidate-cover".into());
        assert_eq!(work.detail().cover_image_url(), Some("old-cover"));
    }
}
