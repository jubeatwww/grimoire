use super::{MoveBatchId, MoveBatchStatus};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum MoveBatchError {
    #[error("only a finished batch can be undone")]
    NotFinished(MoveBatchId),
    #[error("batch {batch} is not {}", expected.as_str())]
    WrongState {
        batch: MoveBatchId,
        expected: MoveBatchStatus,
    },
    #[error("the batch has no op {0}")]
    UnknownOp(i32),
    #[error("op {0} is not waiting for an outcome")]
    OpNotPending(i32),
}

impl ErrorCode for MoveBatchError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFinished(_) => "move_batch.not_finished",
            Self::WrongState { .. } => "move_batch.wrong_state",
            Self::UnknownOp(_) => "move_batch.unknown_op",
            Self::OpNotPending(_) => "move_batch.op_not_pending",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFinished(_) => ErrorKind::Conflict,
            Self::WrongState { .. } | Self::UnknownOp(_) | Self::OpNotPending(_) => {
                ErrorKind::Internal
            }
        }
    }
}
