use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::game_work::{DetailPrecedence, GameWork};
use crate::domain::metadata_source::{
    MetadataProvider, MetadataSource, MetadataSourceError, ProductDetail,
};
use crate::domain::work_source_data::WorkSourceData;
use crate::kernel::Clock;

/// Reading a work's page from a site and taking it into the work: the steps confirm, link and
/// refresh share.
pub(crate) struct GameWorkEnrichmentAppService {
    sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
    clock: Arc<dyn Clock>,
}

impl GameWorkEnrichmentAppService {
    pub(crate) fn new(
        sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { sources, clock }
    }

    /// The site and its work id in a pasted id or URL, the most specific site first.
    pub(crate) fn recognize(&self, input: &str) -> Option<(MetadataProvider, String)> {
        MetadataProvider::RECOGNITION_ORDER
            .into_iter()
            .find_map(|provider| {
                let source = self.sources.get(&provider)?;
                source.recognize(input).map(|id| (provider, id))
            })
    }

    /// The work's page on the site; `None` when the site has no record.
    pub(crate) async fn fetch(
        &self,
        provider: MetadataProvider,
        external_id: &str,
    ) -> Result<Option<ProductDetail>, MetadataSourceError> {
        match self.sources.get(&provider) {
            Some(source) => source.find_detail(external_id).await,
            None => Ok(None),
        }
    }

    /// Takes the site's page into the work and returns the site's own tags and work type, to be
    /// recorded together with the work.
    pub(crate) fn apply(
        &self,
        work: &mut GameWork,
        provider: MetadataProvider,
        detail: ProductDetail,
        precedence: DetailPrecedence,
    ) -> WorkSourceData {
        let at = self.clock.now();
        let (data, _recorded) = WorkSourceData::record(work.id(), provider, &detail, at);
        let _enriched = work.apply_detail(detail, precedence, at);
        data
    }
}
