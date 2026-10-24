use grimoire_app::{
    dlsite::DlsiteSource, metadata_source::ProductDetail, steam::SteamSource, vndb::VndbSource,
};

/// The metadata sources a game work can be linked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src {
    Dlsite,
    Vndb,
    Steam,
}

impl Src {
    pub const ALL: [Src; 3] = [Src::Dlsite, Src::Vndb, Src::Steam];

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "dlsite" => Some(Self::Dlsite),
            "vndb" => Some(Self::Vndb),
            "steam" => Some(Self::Steam),
            _ => None,
        }
    }

    /// Value stored in `work_source_data.source` and used by the API.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dlsite => "dlsite",
            Self::Vndb => "vndb",
            Self::Steam => "steam",
        }
    }

    /// Column on `game_works` that holds this source's external id.
    pub fn id_column(self) -> &'static str {
        match self {
            Self::Dlsite => "dlsite_work_id",
            Self::Vndb => "vndb_id",
            Self::Steam => "steam_app_id",
        }
    }

    pub async fn fetch_detail(self, id: &str) -> anyhow::Result<Option<ProductDetail>> {
        match self {
            Self::Dlsite => DlsiteSource::new().fetch_product_detail(id).await,
            Self::Vndb => VndbSource::new().fetch_product_detail(id).await,
            Self::Steam => SteamSource::new().fetch_product_detail(id).await,
        }
    }

    pub fn source_url(self, id: &str) -> String {
        match self {
            Self::Dlsite => format!("https://www.dlsite.com/maniax/work/=/product_id/{id}.html"),
            Self::Vndb => format!("https://vndb.org/{id}"),
            Self::Steam => format!("https://store.steampowered.com/app/{id}/"),
        }
    }
}
