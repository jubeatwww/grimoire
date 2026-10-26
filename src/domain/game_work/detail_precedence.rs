/// How a source's page is applied to a work that already has values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailPrecedence {
    /// The user picked this source (link, refresh): its page replaces the work's detail, empty
    /// fields included, and renames the work and its circle.
    Overwrite,
    /// A confirm from search: the page's values replace the work's only where the page has one,
    /// so an earlier source's values survive the gaps. Title and circle stay.
    Merge,
}
