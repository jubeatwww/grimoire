/// Why a planned move cannot run right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveConflict {
    SameTarget,
    HeldByMissingRecord,
    HeldByRecord,
    TargetExists,
    SharedWorkFolder,
}

impl MoveConflict {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::SameTarget => "another file is planned for the same path",
            Self::HeldByMissingRecord => {
                "a missing-file record still holds this path; delete it first"
            }
            Self::HeldByRecord => "another inventory record is already at this path",
            Self::TargetExists => "a file already exists at the target",
            Self::SharedWorkFolder => {
                "another work's title maps to the same folder; change one of the titles"
            }
        }
    }
}
