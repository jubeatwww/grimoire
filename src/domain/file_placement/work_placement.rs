use crate::domain::game_work::GameWorkId;

/// What decides a work's folder: its category, circle and title, and how many of its files the
/// library holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkPlacement {
    work_id: GameWorkId,
    title: String,
    circle: Option<String>,
    category_id: String,
    category_folder: String,
    file_count: usize,
}

impl WorkPlacement {
    /// `file_count`: present, non-excluded files linked to the work.
    pub(crate) fn new(
        work_id: GameWorkId,
        title: String,
        circle: Option<String>,
        category_id: String,
        category_folder: String,
        file_count: usize,
    ) -> Self {
        Self {
            work_id,
            title,
            circle,
            category_id,
            category_folder,
            file_count,
        }
    }

    pub(crate) fn work_id(&self) -> GameWorkId {
        self.work_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn circle(&self) -> Option<&str> {
        self.circle.as_deref()
    }

    pub(crate) fn category_id(&self) -> &str {
        &self.category_id
    }

    pub(crate) fn category_folder(&self) -> &str {
        &self.category_folder
    }

    pub(crate) fn file_count(&self) -> usize {
        self.file_count
    }
}
