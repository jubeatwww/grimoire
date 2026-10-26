use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Identity of an aggregate `E`: `type GameWorkId = Id<GameWork>;`.
///
/// The phantom parameter makes a `GameWorkId` unusable where an `InventoryItemId` is expected, at
/// zero runtime cost. `fn() -> E` keeps `Id<E>` `Send + Sync + Copy` whatever `E` is.
pub(crate) struct Id<E: ?Sized> {
    raw: Uuid,
    kind: PhantomData<fn() -> E>,
}

impl<E: ?Sized> Id<E> {
    pub(crate) const fn from_raw(raw: Uuid) -> Self {
        Self {
            raw,
            kind: PhantomData,
        }
    }

    pub(crate) const fn raw(self) -> Uuid {
        self.raw
    }
}

// Hand-written: a derive would demand `E: Clone`, `E: PartialEq`, … of the aggregate itself.
impl<E: ?Sized> Clone for Id<E> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<E: ?Sized> Copy for Id<E> {}
impl<E: ?Sized> PartialEq for Id<E> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<E: ?Sized> Eq for Id<E> {}
impl<E: ?Sized> PartialOrd for Id<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<E: ?Sized> Ord for Id<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl<E: ?Sized> Hash for Id<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}
impl<E: ?Sized> fmt::Debug for Id<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.raw)
    }
}
impl<E: ?Sized> fmt::Display for Id<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}
impl<E: ?Sized> Serialize for Id<E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}
impl<'de, E: ?Sized> Deserialize<'de> for Id<E> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(Self::from_raw)
    }
}

/// Source of new identities. Application code calls `ids.next()`; domain code receives the id.
pub(crate) trait IdGenerator: Send + Sync {
    fn next_raw(&self) -> Uuid;
}

// A generic method would make the trait unusable as `dyn IdGenerator`; an inherent method on the
// trait object keeps `Arc<dyn IdGenerator>` and still returns a typed id.
impl dyn IdGenerator {
    pub(crate) fn next<E: ?Sized>(&self) -> Id<E> {
        Id::from_raw(self.next_raw())
    }
}

/// Time-ordered UUIDs (version 7). Existing rows keep their version 4 ids.
pub(crate) struct UuidV7IdGenerator;

impl IdGenerator for UuidV7IdGenerator {
    fn next_raw(&self) -> Uuid {
        Uuid::now_v7()
    }
}

#[cfg(test)]
pub(crate) mod fakes {
    use std::sync::atomic::{AtomicU64, Ordering};

    use uuid::Uuid;

    use super::IdGenerator;

    /// Hands out 00…01, 00…02, … so tests can predict ids.
    #[derive(Default)]
    pub(crate) struct FakeIdGenerator(AtomicU64);

    impl IdGenerator for FakeIdGenerator {
        fn next_raw(&self) -> Uuid {
            let n = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            Uuid::from_u128(u128::from(n))
        }
    }
}
