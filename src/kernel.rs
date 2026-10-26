//! Technical vocabulary shared by every layer: identity, time, error codes, HTTP edge helpers.
//! No business concept lives here.

mod clock;
mod error_code;
pub(crate) mod http;
mod id;
mod timestamp;

pub(crate) use clock::{Clock, SystemClock};
pub(crate) use error_code::{ErrorCode, ErrorKind};
pub(crate) use id::{Id, IdGenerator, UuidV7IdGenerator};
pub(crate) use timestamp::Timestamp;

#[cfg(test)]
pub(crate) use clock::fakes::FakeClock;
#[cfg(test)]
pub(crate) use id::fakes::FakeIdGenerator;
