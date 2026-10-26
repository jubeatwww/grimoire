use serde::Serialize;

use crate::domain::game_work::GameWorkId;
use crate::domain::metadata_candidate::{MetadataCandidate, MetadataCandidateId};

#[derive(Debug, Serialize)]
pub(crate) struct SearchMetadataOutput {
    pub(crate) candidates: Vec<MetadataCandidateOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataCandidateOutput {
    id: MetadataCandidateId,
    source_name: &'static str,
    source_work_id: String,
    source_url: String,
    rank: i32,
    title: String,
    circle: Option<String>,
    cover_url: Option<String>,
    work_type: Option<String>,
    intro_short: Option<String>,
}

impl From<&MetadataCandidate> for MetadataCandidateOutput {
    fn from(candidate: &MetadataCandidate) -> Self {
        let found = candidate.source_match();
        Self {
            id: candidate.id(),
            source_name: candidate.provider().name(),
            source_work_id: found.source_work_id().to_owned(),
            source_url: candidate.source_url().to_owned(),
            rank: found.rank(),
            title: found.title().to_owned(),
            circle: found.circle().map(str::to_owned),
            cover_url: found.cover_url().map(str::to_owned),
            work_type: found.work_type().map(str::to_owned),
            intro_short: found.intro().map(str::to_owned),
        }
    }
}

/// The work a file is now linked to, after a confirm, a link or a manual entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinkGameWorkOutput {
    pub(crate) game_work_id: GameWorkId,
}
