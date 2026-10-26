/// A primary category: one top-level folder of the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Category {
    id: String,
    label: String,
    folder_name: String,
    sort_order: i32,
}

impl Category {
    pub(crate) fn new(id: String, label: String, folder_name: String, sort_order: i32) -> Self {
        Self {
            id,
            label,
            folder_name,
            sort_order,
        }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    /// "04 角色扮演(RPG)".
    pub(crate) fn folder_name(&self) -> &str {
        &self.folder_name
    }

    pub(crate) fn sort_order(&self) -> i32 {
        self.sort_order
    }
}
