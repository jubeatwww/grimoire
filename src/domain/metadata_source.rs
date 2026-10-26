//! The external sites a work's metadata comes from.

mod metadata_provider;
mod product_detail;
mod source_match;

use async_trait::async_trait;

pub(crate) use metadata_provider::MetadataProvider;
pub(crate) use product_detail::ProductDetail;
pub(crate) use source_match::SourceMatch;

/// One external site. Wiring holds one per provider.
#[async_trait]
pub(crate) trait MetadataSource: Send + Sync {
    fn provider(&self) -> MetadataProvider;

    /// This site's work id inside a pasted id or URL, if it holds one.
    fn recognize(&self, input: &str) -> Option<String>;

    /// Works matching a title or id, best first.
    async fn search(&self, query: &str) -> Result<Vec<SourceMatch>, MetadataSourceError>;

    /// The work's page. `None` when the site has no record for the id.
    async fn find_detail(
        &self,
        external_id: &str,
    ) -> Result<Option<ProductDetail>, MetadataSourceError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MetadataSourceError {
    #[error("{0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
