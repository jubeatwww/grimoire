/// Where a file stands in the matching queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrganizationStatus {
    Pending,
    Matched,
    Confirmed,
    /// No source had the work.
    NoMatch,
    /// Excluded: not a game.
    Ignored,
}

impl OrganizationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Matched => "matched",
            Self::Confirmed => "confirmed",
            Self::NoMatch => "no_match",
            Self::Ignored => "ignored",
        }
    }
}
