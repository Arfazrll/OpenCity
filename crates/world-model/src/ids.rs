//! Stable generational identifiers (blueprint 2.4).
//!
//! # Why generations
//!
//! Buildings, road nodes, road edges, citizens, vehicles and transit lines are created and
//! destroyed constantly, and their slots get reused. A bare index would then be ambiguous: an
//! undo record, a route cache entry or a "follow this citizen" camera could resolve a stale
//! index to whatever object now occupies that slot. Pairing the index with a generation
//! counter makes a stale reference *detectably* stale instead — [`IdArena::get`] returns
//! `None` rather than the wrong object.
//!
//! That matters most for the two features this project is built around: undo/redo over a
//! command queue (blueprint 2.1) and click-and-follow on individual agents (blueprint 2.5.10).
//!
//! # Determinism
//!
//! Slot reuse is last-in-first-out and iteration is by ascending index, so an arena replays
//! identically given identical operations. No hashing and no address-dependent ordering are
//! involved (blueprint 2.5.2).

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A stable, type-safe handle to a value stored in an [`IdArena`].
///
/// `Id<Building>` and `Id<RoadNode>` are different types, so they cannot be mixed up even
/// though both are two `u32`s at runtime.
pub struct Id<T> {
    index: u32,
    generation: u32,
    // `fn() -> T` rather than `T`, so that `Id<T>` is `Send`, `Sync`, `Copy` and covariant
    // regardless of what `T` is. The marker never holds a value.
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Creates an identifier from its raw parts.
    ///
    /// Intended for deserialisation and tests. Normal code obtains identifiers from
    /// [`IdArena::insert`].
    #[inline]
    #[must_use]
    pub const fn from_raw(index: u32, generation: u32) -> Self {
        Self {
            index,
            generation,
            _marker: PhantomData,
        }
    }

    /// The slot index this identifier refers to.
    #[inline]
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// The generation this identifier was issued in.
    #[inline]
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

// Manual trait impls: deriving would add a spurious `T: Clone + Eq + ...` bound, even though
// the marker holds no `T`.

impl<T> Clone for Id<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> PartialOrd for Id<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Index first, so sorting gives arena order - the deterministic iteration order the
        // simulation relies on.
        (self.index, self.generation).cmp(&(other.index, other.generation))
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({}v{})", self.index, self.generation)
    }
}

/// A slot in an [`IdArena`].
struct Slot<T> {
    /// Incremented every time the slot is vacated, which is what invalidates stale ids.
    generation: u32,
    value: Option<T>,
}

/// A generational arena: a `Vec` whose vacated slots are reused without making old handles
/// ambiguous.
pub struct IdArena<T> {
    slots: Vec<Slot<T>>,
    /// Indices of vacant slots, reused last-in-first-out.
    free: Vec<u32>,
    live: usize,
}

impl<T> IdArena<T> {
    /// Creates an empty arena.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            live: 0,
        }
    }

    /// Creates an empty arena with room for `capacity` values.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free: Vec::new(),
            live: 0,
        }
    }

    /// Number of live values.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live
    }

    /// Whether the arena holds no live values.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Inserts a value and returns its identifier.
    ///
    /// Reuses a vacated slot when one is available, incrementing nothing: the generation was
    /// already bumped on removal.
    pub fn insert(&mut self, value: T) -> Id<T> {
        self.live += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            Id::from_raw(index, slot.generation)
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                value: Some(value),
            });
            Id::from_raw(index, 0)
        }
    }

    /// Returns a reference to the value, or `None` if the identifier is stale or out of range.
    #[inline]
    #[must_use]
    pub fn get(&self, id: Id<T>) -> Option<&T> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value, or `None` if the identifier is stale.
    #[inline]
    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }

    /// Whether the identifier still refers to a live value.
    #[inline]
    #[must_use]
    pub fn contains(&self, id: Id<T>) -> bool {
        self.get(id).is_some()
    }

    /// Removes and returns the value, invalidating every outstanding identifier for that slot.
    ///
    /// Returns `None` if the identifier was already stale, so a double-remove (easy to produce
    /// with undo/redo) is harmless rather than corrupting.
    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let value = slot.value.take()?;
        // Saturate rather than wrap: after 4 billion reuses of one slot, refusing to reuse it
        // again is far better than handing out an id that collides with an ancient one.
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(id.index);
        }
        self.live -= 1;
        Some(value)
    }

    /// Removes every value, keeping the allocated capacity.
    ///
    /// Generations are bumped, so identifiers issued before the clear stay invalid.
    pub fn clear(&mut self) {
        self.free.clear();
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.take().is_some()
                && let Some(next) = slot.generation.checked_add(1)
            {
                slot.generation = next;
                self.free.push(index as u32);
            }
        }
        self.live = 0;
    }

    /// Iterates over live `(id, &value)` pairs in ascending slot order.
    ///
    /// The order is deterministic and independent of insertion and removal history, which is
    /// what the simulation needs for reproducible results (blueprint 2.5.2).
    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|value| (Id::from_raw(index as u32, slot.generation), value))
        })
    }

    /// Iterates over live `(id, &mut value)` pairs in ascending slot order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id<T>, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let generation = slot.generation;
                slot.value
                    .as_mut()
                    .map(move |value| (Id::from_raw(index as u32, generation), value))
            })
    }

    /// Iterates over live identifiers in ascending slot order.
    pub fn ids(&self) -> impl Iterator<Item = Id<T>> {
        self.iter().map(|(id, _)| id)
    }
}

impl<T> Default for IdArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for IdArena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Building(u32);

    #[test]
    fn inserted_values_are_retrievable() {
        let mut arena = IdArena::new();
        let a = arena.insert(Building(1));
        let b = arena.insert(Building(2));
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(a), Some(&Building(1)));
        assert_eq!(arena.get(b), Some(&Building(2)));
        assert_ne!(a, b);
    }

    #[test]
    fn a_removed_id_does_not_resolve_to_the_value_that_replaces_it() {
        // This is the entire reason generations exist.
        let mut arena = IdArena::new();
        let old = arena.insert(Building(1));
        assert_eq!(arena.remove(old), Some(Building(1)));

        let new = arena.insert(Building(999));
        // The slot was reused...
        assert_eq!(new.index(), old.index());
        // ...but the old handle is dead, and the new one works.
        assert_eq!(arena.get(old), None);
        assert!(!arena.contains(old));
        assert_eq!(arena.get(new), Some(&Building(999)));
        assert!(new.generation() > old.generation());
    }

    #[test]
    fn double_remove_is_harmless() {
        // Undo/redo makes this easy to trigger; it must not corrupt the arena.
        let mut arena = IdArena::new();
        let id = arena.insert(Building(7));
        assert_eq!(arena.remove(id), Some(Building(7)));
        assert_eq!(arena.remove(id), None);
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
    }

    #[test]
    fn out_of_range_and_fabricated_ids_return_none() {
        let mut arena: IdArena<Building> = IdArena::new();
        assert_eq!(arena.get(Id::from_raw(0, 0)), None);
        arena.insert(Building(1));
        assert_eq!(arena.get(Id::from_raw(9_999, 0)), None);
        assert_eq!(arena.get(Id::from_raw(0, 12_345)), None);
        assert_eq!(arena.remove(Id::from_raw(9_999, 0)), None);
    }

    #[test]
    fn mutation_through_a_live_id_works_and_through_a_stale_one_does_not() {
        let mut arena = IdArena::new();
        let id = arena.insert(Building(1));
        if let Some(b) = arena.get_mut(id) {
            b.0 = 42;
        }
        assert_eq!(arena.get(id), Some(&Building(42)));
        arena.remove(id);
        assert!(arena.get_mut(id).is_none());
    }

    #[test]
    fn slots_are_reused_so_the_arena_does_not_grow_without_bound() {
        let mut arena = IdArena::new();
        let mut ids = Vec::new();
        for i in 0..100 {
            ids.push(arena.insert(Building(i)));
        }
        for id in &ids {
            arena.remove(*id);
        }
        assert_eq!(arena.len(), 0);
        for i in 0..100 {
            arena.insert(Building(i));
        }
        assert_eq!(arena.len(), 100);
        // Every index stayed inside the original range: no slot leak.
        assert!(arena.ids().all(|id| id.index() < 100));
    }

    #[test]
    fn iteration_is_in_ascending_slot_order_regardless_of_history() {
        let mut arena = IdArena::new();
        let a = arena.insert(Building(0));
        let b = arena.insert(Building(1));
        let c = arena.insert(Building(2));
        // Vacate the middle, then refill it. A hash-based container would now iterate in an
        // arbitrary order; this one must not.
        arena.remove(b);
        let d = arena.insert(Building(99));
        assert_eq!(d.index(), b.index());

        let order: Vec<u32> = arena.ids().map(|id| id.index()).collect();
        assert_eq!(order, vec![0, 1, 2]);
        let values: Vec<u32> = arena.iter().map(|(_, v)| v.0).collect();
        assert_eq!(values, vec![0, 99, 2]);
        assert!(arena.contains(a) && arena.contains(c) && arena.contains(d));
    }

    #[test]
    fn iteration_yields_ids_that_resolve_back_to_the_same_values() {
        let mut arena = IdArena::new();
        for i in 0..10 {
            arena.insert(Building(i));
        }
        let fourth = arena.ids().nth(3).expect("ten live values");
        arena.remove(fourth);
        let pairs: Vec<(Id<Building>, u32)> = arena.iter().map(|(id, v)| (id, v.0)).collect();
        assert_eq!(pairs.len(), 9);
        for (id, value) in pairs {
            assert_eq!(arena.get(id).map(|b| b.0), Some(value));
        }
    }

    #[test]
    fn iter_mut_can_modify_every_live_value() {
        let mut arena = IdArena::new();
        for i in 0..5 {
            arena.insert(Building(i));
        }
        for (_, b) in arena.iter_mut() {
            b.0 *= 10;
        }
        let values: Vec<u32> = arena.iter().map(|(_, v)| v.0).collect();
        assert_eq!(values, vec![0, 10, 20, 30, 40]);
    }

    #[test]
    fn clear_invalidates_every_outstanding_id() {
        let mut arena = IdArena::new();
        let a = arena.insert(Building(1));
        let b = arena.insert(Building(2));
        arena.clear();
        assert!(arena.is_empty());
        assert_eq!(arena.get(a), None);
        assert_eq!(arena.get(b), None);
        // And the slots are available again.
        let c = arena.insert(Building(3));
        assert_eq!(arena.get(c), Some(&Building(3)));
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn ids_are_small_and_cheap_to_copy() {
        // Blueprint 2.4 specifies u32 index + u32 generation. The phantom marker must not add
        // anything to that.
        assert_eq!(size_of::<Id<Building>>(), 8);
        assert_eq!(size_of::<Option<Id<Building>>>(), 12);
    }

    #[test]
    fn ids_sort_by_index_then_generation() {
        let mut v = vec![
            Id::<Building>::from_raw(2, 0),
            Id::from_raw(1, 5),
            Id::from_raw(1, 2),
        ];
        v.sort();
        assert_eq!(
            v,
            vec![Id::from_raw(1, 2), Id::from_raw(1, 5), Id::from_raw(2, 0)]
        );
    }

    #[test]
    fn debug_output_is_compact() {
        assert_eq!(format!("{:?}", Id::<Building>::from_raw(7, 3)), "Id(7v3)");
    }
}
