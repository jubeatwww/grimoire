/// How a failure should be treated by whoever receives it. Presentation maps it to a status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    /// The request itself is malformed.
    Invalid,
    NotFound,
    /// The request collides with current state (duplicate, wrong state).
    Conflict,
    /// A business rule refused the request.
    #[allow(dead_code, reason = "kernel vocabulary; no rule refuses a request yet")]
    Rejected,
    /// A dependency is down; retrying later may succeed.
    Unavailable,
    Internal,
}

/// Implemented by every use-case error and every domain error. The code is a stable,
/// machine-readable contract (`"move_batch.not_finished"`) written in the domain's own language.
pub(crate) trait ErrorCode: std::error::Error {
    fn code(&self) -> &'static str;
    fn kind(&self) -> ErrorKind;
}
