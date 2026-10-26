//! The web UI's HTTP API, one module per application scenario. Paths are kept from before the
//! scenario split, so several modules share a prefix such as `/api/metadata`.

mod asset_client;
mod catalog_client;
mod classification_client;
mod library_client;
mod organization_client;
mod source_refetch_client;
mod vocabulary_client;

pub(crate) use asset_client::{AssetClientHandlers, routes as asset_client_routes};
pub(crate) use catalog_client::{CatalogClientHandlers, routes as catalog_client_routes};
pub(crate) use classification_client::{
    ClassificationClientHandlers, routes as classification_client_routes,
};
pub(crate) use library_client::{LibraryClientHandlers, routes as library_client_routes};
pub(crate) use organization_client::{
    OrganizationClientHandlers, routes as organization_client_routes,
};
pub(crate) use source_refetch_client::{
    SourceRefetchClientHandlers, routes as source_refetch_client_routes,
};
pub(crate) use vocabulary_client::{VocabularyClientHandlers, routes as vocabulary_client_routes};
