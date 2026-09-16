//! DMX universe management.

use crate::dmx::DmxUniverse;
use std::collections::HashMap;

/// A map of universe number → universe, creating universes on demand.
#[derive(Debug, Clone, Default)]
pub struct UniverseManager {
    universes: HashMap<u16, DmxUniverse>,
}

impl UniverseManager {
    /// An empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the universe, creating an all-black one if absent.
    pub fn get_or_create(&mut self, universe: u16) -> &mut DmxUniverse {
        self.universes
            .entry(universe)
            .or_insert_with(|| DmxUniverse::new(universe))
    }

    /// Borrowed access without creating.
    pub fn get(&self, universe: u16) -> Option<&DmxUniverse> {
        self.universes.get(&universe)
    }

    /// Mutable access without creating.
    pub fn get_mut(&mut self, universe: u16) -> Option<&mut DmxUniverse> {
        self.universes.get_mut(&universe)
    }

    /// Replaces or inserts a universe.
    pub fn insert(&mut self, universe: DmxUniverse) {
        self.universes.insert(universe.universe, universe);
    }

    /// Merges an incoming universe's slot data into the manager (e.g. when
    /// receiving from the network).
    pub fn merge(&mut self, incoming: &DmxUniverse) {
        self.get_or_create(incoming.universe).channels = incoming.channels;
    }

    /// Number of managed universes.
    pub fn len(&self) -> usize {
        self.universes.len()
    }

    /// Whether no universes are managed.
    pub fn is_empty(&self) -> bool {
        self.universes.is_empty()
    }

    /// Iterates over managed universes in universe-number order.
    pub fn iter(&self) -> impl Iterator<Item = &DmxUniverse> {
        let mut sorted: Vec<&DmxUniverse> = self.universes.values().collect();
        sorted.sort_by_key(|u| u.universe);
        sorted.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_on_demand() {
        let mut m = UniverseManager::new();
        assert!(m.is_empty());
        m.get_or_create(3).set_channel(0, 99);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(3).unwrap().get_channel(0), 99);
        // Re-fetch is the same universe.
        assert_eq!(m.get_or_create(3).get_channel(0), 99);
    }

    #[test]
    fn merge_and_iterate_sorted() {
        let mut m = UniverseManager::new();
        for n in [7u16, 1, 4] {
            let mut u = DmxUniverse::new(n);
            u.set_channel(0, n as u8);
            m.insert(u);
        }
        let order: Vec<u16> = m.iter().map(|u| u.universe).collect();
        assert_eq!(order, vec![1, 4, 7]);
        let mut incoming = DmxUniverse::new(4);
        incoming.set_channel(1, 42);
        m.merge(&incoming);
        assert_eq!(m.get(4).unwrap().get_channel(1), 42);
    }
}
