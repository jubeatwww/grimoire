use serde::{Deserialize, Serialize};

/// A site a game work can be linked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MetadataProvider {
    Dlsite,
    Vndb,
    Steam,
}

impl MetadataProvider {
    pub(crate) const ALL: [MetadataProvider; 3] = [Self::Dlsite, Self::Vndb, Self::Steam];

    /// The order a pasted id or URL is tried in, most specific first: a Steam store URL, then a
    /// VNDB `vN` token or URL, then a DLsite RJ / VJ / BJ code or URL.
    pub(crate) const RECOGNITION_ORDER: [MetadataProvider; 3] =
        [Self::Steam, Self::Vndb, Self::Dlsite];

    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "dlsite" => Some(Self::Dlsite),
            "vndb" => Some(Self::Vndb),
            "steam" => Some(Self::Steam),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Dlsite => "dlsite",
            Self::Vndb => "vndb",
            Self::Steam => "steam",
        }
    }

    /// The work's page on the site.
    pub(crate) fn work_url(self, external_id: &str) -> String {
        match self {
            Self::Dlsite => {
                format!("https://www.dlsite.com/maniax/work/=/product_id/{external_id}.html")
            }
            Self::Vndb => format!("https://vndb.org/{external_id}"),
            Self::Steam => format!("https://store.steampowered.com/app/{external_id}/"),
        }
    }
}
