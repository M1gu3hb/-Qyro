//! The handle table. Specification: `docs/adr/ADR-0032-engine-ffi.md` §4.
//!
//! A handle is a `u64`, never a pointer. A corrupt, repeated or fabricated
//! pointer makes Rust dereference an address the caller influences; a corrupt
//! integer can only fail a lookup, and a failed lookup is a typed error. Nothing
//! in this file dereferences anything the caller supplied, and nothing in it can
//! panic on a handle it is handed — which is the property the four tests at the
//! bottom exist to hold onto.
//!
//! Layout: `generation: u32` in the high half, `slot: u32` in the low half.
//! Both halves are load-bearing and neither is enough alone. A non-reusing
//! counter alone grows without bound for the life of the process; a slot alone
//! cannot tell a live handle from a stale one naming a reused slot.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// The table's capacity. ADR-0032 §4.
///
/// This is what bounds the deliberate leak: a handle Dart loses without closing
/// is never reclaimed, because a sweep needs a liveness signal Dart cannot give
/// and a sweep that closes a session Dart is still using is worse than the leak.
/// With a cap, losing handles produces a typed error rather than growth.
pub const MAX_ESTABLISHED_SESSIONS: usize = 4;

/// Why a handle did not resolve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleError {
    /// The handle does not name a live session.
    ///
    /// Deliberately one variant covering all three ways resolution fails --
    /// out-of-range slot, empty slot, mismatched generation. ADR-0032 §4 freezes
    /// double-close as *the generation check*, and "this handle is not live" is
    /// a single fact about the world: splitting it would invite a caller to
    /// branch on the difference between a handle that was never valid and one
    /// that stopped being valid, which is not a difference they can act on.
    NotLive,
    /// The table is full: `MAX_ESTABLISHED_SESSIONS` sessions are already open.
    Full,
}

/// One entry in the table.
enum Slot<T> {
    /// Free, and carrying the generation the next occupant will be given.
    Empty {
        next_generation: u32,
    },
    Live {
        generation: u32,
        value: T,
    },
    /// Permanently withdrawn: its generation counter overflowed.
    ///
    /// After 2^32 closes on one slot the generation would wrap and a very old
    /// handle would become valid again. Retiring the slot costs one entry of
    /// capacity and closes that door for good.
    Retired,
}

/// A table of live sessions addressed by opaque integer handles.
pub struct HandleTable<T> {
    slots: Vec<Slot<T>>,
}

impl<T> Default for HandleTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> HandleTable<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: (0..MAX_ESTABLISHED_SESSIONS)
                // Generations start at 1, which is what makes the handle 0
                // invalid by construction rather than by a special case.
                .map(|_| Slot::Empty { next_generation: 1 })
                .collect(),
        }
    }

    /// Splits a handle into its generation and slot halves.
    const fn split(handle: u64) -> (u32, u32) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the truncation is the point: the low half is the slot"
        )]
        ((handle >> 32) as u32, handle as u32)
    }

    const fn compose(generation: u32, slot: u32) -> u64 {
        ((generation as u64) << 32) | (slot as u64)
    }

    /// Stores `value` and returns its handle.
    ///
    /// # Errors
    ///
    /// [`HandleError::Full`] when every slot is live or retired.
    pub fn insert(&mut self, value: T) -> Result<u64, HandleError> {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if let Slot::Empty { next_generation } = *slot {
                *slot = Slot::Live {
                    generation: next_generation,
                    value,
                };
                let Ok(index) = u32::try_from(index) else {
                    // Unreachable while MAX_ESTABLISHED_SESSIONS is 4, and
                    // written as a branch rather than an assertion so that
                    // raising the cap cannot turn it into a panic.
                    return Err(HandleError::Full);
                };
                return Ok(Self::compose(next_generation, index));
            }
        }
        Err(HandleError::Full)
    }

    /// Resolves a handle to a shared borrow.
    ///
    /// The three checks run in the order ADR-0032 §4 freezes: slot out of range,
    /// then slot empty, then generation mismatch.
    ///
    /// # Errors
    ///
    /// [`HandleError::NotLive`] if the handle does not name a live session.
    pub fn get(&self, handle: u64) -> Result<&T, HandleError> {
        let (generation, slot) = Self::split(handle);
        let slot = usize::try_from(slot).map_err(|_| HandleError::NotLive)?;
        match self.slots.get(slot) {
            Some(&Slot::Live {
                generation: live,
                ref value,
            }) if live == generation => Ok(value),
            _ => Err(HandleError::NotLive),
        }
    }

    /// Resolves a handle to a unique borrow.
    ///
    /// # Errors
    ///
    /// [`HandleError::NotLive`] if the handle does not name a live session.
    pub fn get_mut(&mut self, handle: u64) -> Result<&mut T, HandleError> {
        let (generation, slot) = Self::split(handle);
        let slot = usize::try_from(slot).map_err(|_| HandleError::NotLive)?;
        match self.slots.get_mut(slot) {
            Some(&mut Slot::Live {
                generation: live,
                ref mut value,
            }) if live == generation => Ok(value),
            _ => Err(HandleError::NotLive),
        }
    }

    /// Closes a handle, returning what it held.
    ///
    /// Bumps the slot's generation, so the handle just closed can never resolve
    /// again — which is all double-close protection is.
    ///
    /// # Errors
    ///
    /// [`HandleError::NotLive`] if the handle does not name a live session.
    pub fn remove(&mut self, handle: u64) -> Result<T, HandleError> {
        let (generation, index) = Self::split(handle);
        let index = usize::try_from(index).map_err(|_| HandleError::NotLive)?;
        let slot = self.slots.get_mut(index).ok_or(HandleError::NotLive)?;

        let live = match *slot {
            Slot::Live {
                generation: live, ..
            } if live == generation => live,
            _ => return Err(HandleError::NotLive),
        };

        // Retire rather than wrap. `checked_add` is the whole overflow policy.
        let replacement = match live.checked_add(1) {
            Some(next_generation) => Slot::Empty { next_generation },
            None => Slot::Retired,
        };
        match core::mem::replace(slot, replacement) {
            Slot::Live { value, .. } => Ok(value),
            // Cannot happen: the match above already established this slot is
            // live. Written as a branch because this crate denies `unreachable`,
            // and a lookup failure is the correct shape for "not live" anyway.
            Slot::Empty { .. } | Slot::Retired => Err(HandleError::NotLive),
        }
    }

    /// How many sessions are live. For tests and for the capacity error.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Live { .. }))
            .count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]
    use super::{HandleError, HandleTable, MAX_ESTABLISHED_SESSIONS, Slot};

    #[test]
    fn a_double_close_is_an_error_and_not_a_crash() {
        let mut table = HandleTable::new();
        let handle = table.insert("session").expect("the table starts empty");

        assert_eq!(table.remove(handle), Ok("session"));
        assert_eq!(
            table.remove(handle),
            Err(HandleError::NotLive),
            "the second close must be a typed error"
        );
        assert_eq!(
            table.get(handle),
            Err(HandleError::NotLive),
            "and the handle must not read either"
        );

        // And the mechanism, not just the outcome. ADR-0032 §4 says double-close
        // *is* the generation check; in this implementation it is not, and the
        // difference matters. Once the slot is empty, resolution fails on the
        // emptiness before it ever compares generations -- so deleting the
        // generation bump leaves the three assertions above still passing, and
        // only breaks when the slot is reused (QYR-0307).
        //
        // Asserting the bump here is what makes this test cover the control its
        // name claims.
        let reopened = table.insert("another").expect("the slot came free");
        assert_eq!(
            reopened & 0xFFFF_FFFF,
            handle & 0xFFFF_FFFF,
            "the same slot, or this assertion is about something else"
        );
        assert_eq!(
            reopened >> 32,
            (handle >> 32) + 1,
            "close must advance the generation, or a stale handle outlives its session"
        );
    }

    #[test]
    fn an_invalid_handle_is_refused_by_name() {
        let table: HandleTable<&str> = HandleTable::new();

        // Slot beyond the table. The first of the three checks.
        assert_eq!(
            table.get(super::HandleTable::<&str>::compose(1, 9999)),
            Err(HandleError::NotLive)
        );
        // Live generation, but the slot was never filled. The second.
        assert_eq!(
            table.get(super::HandleTable::<&str>::compose(1, 0)),
            Err(HandleError::NotLive)
        );
        // Every bit set, the shape a corrupt integer arrives in.
        assert_eq!(table.get(u64::MAX), Err(HandleError::NotLive));
    }

    #[test]
    fn a_handle_from_another_session_does_not_resolve_to_the_one_living_in_its_slot() {
        // The reason the generation half exists. Without it, `stale` and the
        // handle for `second` are the same 64 bits, and the first session's
        // handle would read and close a session it never opened.
        let mut table = HandleTable::new();
        let stale = table.insert("first").expect("the table starts empty");
        table.remove(stale).expect("the first session is live");

        let second = table.insert("second").expect("a slot came free");

        assert_eq!(
            stale & 0xFFFF_FFFF,
            second & 0xFFFF_FFFF,
            "this test is only meaningful if the slot really was reused"
        );
        assert_ne!(stale, second, "and the generation half must differ");

        assert_eq!(table.get(stale), Err(HandleError::NotLive));
        assert_eq!(table.remove(stale), Err(HandleError::NotLive));
        assert_eq!(
            table.get(second),
            Ok(&"second"),
            "while the live handle still works"
        );
    }

    #[test]
    fn the_handle_zero_is_refused_because_generations_start_at_one() {
        let mut table = HandleTable::new();

        assert_eq!(
            table.get(0),
            Err(HandleError::NotLive),
            "zero is the likeliest accidental value from Dart"
        );
        assert_eq!(table.remove(0), Err(HandleError::NotLive));

        // And the property is checked, not assumed: no live handle can *be*
        // zero, because zero would need generation 0 and generations start at 1.
        let handle = table.insert("session").expect("the table starts empty");
        assert_ne!(handle, 0);
        assert_eq!(handle >> 32, 1, "the first generation is 1, not 0");
    }

    #[test]
    fn get_mut_resolves_by_the_same_rules_as_get() {
        // Added after the sweep: `get_mut` had no caller in any test, so its
        // generation check could be deleted, inverted, or forced either way and
        // nothing noticed. It is the accessor every session operation uses.
        let mut table = HandleTable::new();
        let handle = table.insert(String::from("live")).expect("empty table");

        table
            .get_mut(handle)
            .expect("a live handle resolves")
            .push_str(" and mutated");
        assert_eq!(table.get(handle), Ok(&String::from("live and mutated")));

        let stale = handle;
        table.remove(handle).expect("live");
        let reused = table
            .insert(String::from("second"))
            .expect("slot came free");
        assert_eq!(
            stale & 0xFFFF_FFFF,
            reused & 0xFFFF_FFFF,
            "only meaningful if the slot was reused"
        );
        assert_eq!(
            table.get_mut(stale),
            Err(HandleError::NotLive),
            "get_mut must refuse a stale generation, exactly as get does"
        );
        assert_eq!(table.get_mut(0), Err(HandleError::NotLive));
        assert_eq!(table.get_mut(u64::MAX), Err(HandleError::NotLive));
        assert!(table.get_mut(reused).is_ok());
    }

    #[test]
    fn len_and_is_empty_track_what_the_table_holds() {
        // Also added after the sweep: `is_empty` had no caller, so both its
        // return value and its comparison could be replaced freely.
        let mut table = HandleTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);

        let first = table.insert(1).expect("empty table");
        assert!(
            !table.is_empty(),
            "a table holding one session is not empty"
        );
        assert_eq!(table.len(), 1);

        let second = table.insert(2).expect("within capacity");
        assert_eq!(table.len(), 2);

        table.remove(first).expect("live");
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        table.remove(second).expect("live");
        assert!(table.is_empty(), "and empty again once both are closed");
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn the_two_halves_of_a_handle_do_not_overlap() {
        // The sweep flags `|` -> `^` in `compose` as surviving, and it always
        // will: the generation occupies the high 32 bits and the slot the low
        // 32, so the operands share no set bit and the two operators are
        // identical by construction. That is an equivalent mutant, not a gap --
        // and this test is what makes the claim checkable rather than asserted.
        let mut table = HandleTable::new();
        let handle = table.insert("session").expect("empty table");
        let (generation, slot) = HandleTable::<&str>::split(handle);

        assert_eq!(
            HandleTable::<&str>::compose(generation, slot),
            handle,
            "split and compose must round-trip"
        );
        assert_eq!(
            (u64::from(generation) << 32) & u64::from(slot),
            0,
            "the halves share no bit, which is why | and ^ agree here"
        );
        assert_eq!(
            HandleTable::<&str>::compose(u32::MAX, u32::MAX),
            u64::MAX,
            "and the composition covers the whole width"
        );
    }

    #[test]
    fn the_table_refuses_a_fifth_session_instead_of_growing() {
        let mut table = HandleTable::new();
        for index in 0..MAX_ESTABLISHED_SESSIONS {
            table.insert(index).expect("within capacity");
        }
        assert_eq!(table.len(), MAX_ESTABLISHED_SESSIONS);
        assert_eq!(
            table.insert(99),
            Err(HandleError::Full),
            "the bounded leak is bounded by this refusal"
        );
    }

    #[test]
    fn a_slot_whose_generation_would_wrap_is_retired_rather_than_reused() {
        let mut table: HandleTable<&str> = HandleTable::new();
        // Drive one slot to the last generation it can hold. Reaching this by
        // 2^32 real closes is not testable, so the state is set directly --
        // which is the point of the branch being `checked_add` and not a count.
        *table.slots.first_mut().expect("the table has slots") = Slot::Live {
            generation: u32::MAX,
            value: "last",
        };
        let handle = HandleTable::<&str>::compose(u32::MAX, 0);

        assert_eq!(table.remove(handle), Ok("last"));
        assert!(
            matches!(table.slots.first(), Some(Slot::Retired)),
            "the slot must be withdrawn, not handed back with a wrapped counter"
        );

        // The retired slot is gone from capacity, and no handle names it.
        assert_eq!(table.get(handle), Err(HandleError::NotLive));
        assert_eq!(
            table.get(HandleTable::<&str>::compose(1, 0)),
            Err(HandleError::NotLive)
        );
        for index in 0..MAX_ESTABLISHED_SESSIONS - 1 {
            table.insert("filler").expect("the other slots still work");
            let _ = index;
        }
        assert_eq!(
            table.insert("one too many"),
            Err(HandleError::Full),
            "capacity dropped by exactly the retired slot"
        );
    }
}
