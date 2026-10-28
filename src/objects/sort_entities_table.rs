//! SortEntitiesTable object implementation.
//!
//! Controls the draw order of entities within a block reference.

use crate::tables::BlockRecord;
use crate::types::Handle;
use std::collections::HashMap;

// ============================================================================
// SortEntsEntry
// ============================================================================

/// An entry in the sort entities table.
///
/// Maps an entity handle to its sort handle for determining draw order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortEntsEntry {
    /// Entity handle (the entity being sorted).
    /// DXF code: 331
    pub entity_handle: Handle,

    /// Sort handle (determines draw order).
    /// Lower values are drawn first (below).
    /// DXF code: 5
    pub sort_handle: Handle,
}

impl SortEntsEntry {
    /// Creates a new sort entry.
    pub fn new(entity_handle: Handle, sort_handle: Handle) -> Self {
        SortEntsEntry {
            entity_handle,
            sort_handle,
        }
    }
}

// ============================================================================
// SortEntitiesTable
// ============================================================================

/// Sort entities table object.
///
/// Controls the draw order of entities within a block reference (including model space).
/// Entities with lower sort handles are drawn first (appear below entities with higher handles).
///
/// # DXF Information
/// - Object type: SORTENTSTABLE
/// - Subclass marker: AcDbSortentsTable
/// - Storage: In the block's extension dictionary under "ACAD_SORTENTS"
///
/// # Draw Order
/// - Entities not in the table use their own handle for sort order
/// - Entities in the table use the specified sort handle
/// - Lower sort handles are drawn first (below higher handles)
///
/// # Example
///
/// ```ignore
/// use acadrust::objects::SortEntitiesTable;
/// use acadrust::types::Handle;
///
/// let mut table = SortEntitiesTable::new();
/// table.block_owner_handle = Handle::new(0x1F); // Model space block
///
/// // Move entity 0x100 to back (draw first)
/// table.add_entry(Handle::new(0x100), Handle::new(0x1));
///
/// // Move entity 0x101 to front (draw last)
/// table.add_entry(Handle::new(0x101), Handle::new(0xFFFFFF));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortEntitiesTable {
    /// Object handle.
    pub handle: Handle,

    /// Owner handle (extension dictionary).
    pub owner_handle: Handle,

    /// Block owner handle (the block record that owns the entities).
    /// DXF code: 330
    pub block_owner_handle: Handle,

    /// Sort entries mapping entity handles to sort handles.
    entries: Vec<SortEntsEntry>,

    /// Lookup map for fast entity handle searches.
    entry_map: HashMap<u64, usize>,

    /// Original DXF group codes for exact-version round-trips.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw_dxf_codes: Option<Vec<(i32, String)>>,

    /// DXF version that produced `raw_dxf_codes`.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub raw_dxf_version: Option<crate::types::DxfVersion>,
}

enum Placement {
    Back,
    Front,
    Above(Handle),
    Below(Handle),
}

impl SortEntitiesTable {
    /// Object type name.
    pub const OBJECT_NAME: &'static str = "SORTENTSTABLE";

    /// Subclass marker.
    pub const SUBCLASS_MARKER: &'static str = "AcDbSortentsTable";

    /// Dictionary entry name.
    pub const DICTIONARY_KEY: &'static str = "ACAD_SORTENTS";

    /// Creates a new empty SortEntitiesTable.
    pub fn new() -> Self {
        SortEntitiesTable {
            handle: Handle::NULL,
            owner_handle: Handle::NULL,
            block_owner_handle: Handle::NULL,
            entries: Vec::new(),
            entry_map: HashMap::new(),
            raw_dxf_codes: None,
            raw_dxf_version: None,
        }
    }

    /// Creates a SortEntitiesTable for a specific block.
    pub fn for_block(block_handle: Handle) -> Self {
        SortEntitiesTable {
            block_owner_handle: block_handle,
            ..Self::new()
        }
    }

    /// Returns the number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the table has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Adds or updates a sort entry.
    ///
    /// If the entity already has an entry, its sort handle is updated.
    /// Returns the previous sort handle if the entity was already in the table.
    pub fn add_entry(&mut self, entity_handle: Handle, sort_handle: Handle) -> Option<Handle> {
        let key = entity_handle.value();

        if let Some(&idx) = self.entry_map.get(&key) {
            // Update existing entry
            let old_handle = self.entries[idx].sort_handle;
            self.entries[idx].sort_handle = sort_handle;
            Some(old_handle)
        } else {
            // Add new entry
            let idx = self.entries.len();
            self.entries
                .push(SortEntsEntry::new(entity_handle, sort_handle));
            self.entry_map.insert(key, idx);
            None
        }
    }

    /// Removes an entity from the sort table.
    ///
    /// Returns the entry if it was found and removed.
    pub fn remove_entry(&mut self, entity_handle: Handle) -> Option<SortEntsEntry> {
        let key = entity_handle.value();

        if let Some(&idx) = self.entry_map.get(&key) {
            // Remove from map first
            self.entry_map.remove(&key);

            // Remove from entries (swap remove for efficiency)
            let entry = self.entries.swap_remove(idx);

            // Update the map for the swapped element (if any)
            if idx < self.entries.len() {
                let swapped_key = self.entries[idx].entity_handle.value();
                self.entry_map.insert(swapped_key, idx);
            }

            Some(entry)
        } else {
            None
        }
    }

    /// Gets the sort handle for an entity.
    ///
    /// Returns None if the entity is not in the table.
    pub fn get_sort_handle(&self, entity_handle: Handle) -> Option<Handle> {
        let key = entity_handle.value();
        self.entry_map
            .get(&key)
            .map(|&idx| self.entries[idx].sort_handle)
    }

    /// Checks if an entity is in the sort table.
    pub fn contains(&self, entity_handle: Handle) -> bool {
        self.entry_map.contains_key(&entity_handle.value())
    }

    /// Returns an iterator over all entries.
    pub fn entries(&self) -> impl Iterator<Item = &SortEntsEntry> {
        self.entries.iter()
    }

    /// Returns a mutable iterator over all entries.
    pub fn entries_mut(&mut self) -> impl Iterator<Item = &mut SortEntsEntry> {
        self.entries.iter_mut()
    }

    /// Returns entries sorted by sort handle (draw order).
    ///
    /// Entries with lower sort handles come first.
    pub fn sorted_entries(&self) -> Vec<&SortEntsEntry> {
        let mut sorted: Vec<_> = self.entries.iter().collect();
        sorted.sort_by_key(|e| e.sort_handle.value());
        sorted
    }

    /// Clears all entries from the table.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.entry_map.clear();
    }

    /// Sends an entity behind every member of its owning block, including unlisted entities.
    /// Returns the assigned key, or None for the wrong block or a missing entity.
    pub fn send_to_back(&mut self, entity_handle: Handle, block: &BlockRecord) -> Option<Handle> {
        self.reorder(entity_handle, block, Placement::Back)
    }

    /// Brings an entity in front of every member of its owning block, including unlisted entities.
    /// Returns the assigned key, or None for the wrong block or a missing entity.
    pub fn bring_to_front(&mut self, entity_handle: Handle, block: &BlockRecord) -> Option<Handle> {
        self.reorder(entity_handle, block, Placement::Front)
    }

    /// Moves an entity immediately above another member of the owning block.
    /// Returns None for the wrong block or if either entity is absent from that block.
    pub fn move_above(
        &mut self,
        entity_handle: Handle,
        target_handle: Handle,
        block: &BlockRecord,
    ) -> Option<Handle> {
        self.reorder(entity_handle, block, Placement::Above(target_handle))
    }

    /// Moves an entity immediately below another member of the owning block.
    /// Returns None for the wrong block or if either entity is absent from that block.
    pub fn move_below(
        &mut self,
        entity_handle: Handle,
        target_handle: Handle,
        block: &BlockRecord,
    ) -> Option<Handle> {
        self.reorder(entity_handle, block, Placement::Below(target_handle))
    }

    fn reorder(
        &mut self,
        entity: Handle,
        block: &BlockRecord,
        placement: Placement,
    ) -> Option<Handle> {
        if self.block_owner_handle != block.handle || !block.entity_handles.contains(&entity) {
            return None;
        }
        let mut order = block.entity_handles.clone();
        order.sort_by_key(|h| (self.effective_sort_handle(*h).value(), h.value()));
        // Moving relative to oneself changes nothing, including imported partial-table keys.
        if matches!(placement, Placement::Above(target) | Placement::Below(target) if target == entity)
        {
            return Some(self.effective_sort_handle(entity));
        }
        order.retain(|h| *h != entity);
        let at = match placement {
            Placement::Back => 0,
            Placement::Front => order.len(),
            Placement::Above(target) => order.iter().position(|h| *h == target)? + 1,
            Placement::Below(target) => order.iter().position(|h| *h == target)?,
        };
        order.insert(at, entity);
        // A space-wide permutation needs no arithmetic gaps and cannot overflow or tie keys.
        // Imported key values remain untouched until an explicit native ordering operation.
        let mut keys = order.clone();
        keys.sort_unstable_by_key(|h| h.value());
        self.clear();
        for (entity, key) in order.into_iter().zip(keys) {
            self.add_entry(entity, key);
        }
        self.get_sort_handle(entity)
    }

    /// Rebuilds the entry map from the entries vector.
    ///
    /// Call this after modifying entries directly.
    pub fn rebuild_map(&mut self) {
        self.entry_map.clear();
        for (idx, entry) in self.entries.iter().enumerate() {
            self.entry_map.insert(entry.entity_handle.value(), idx);
        }
    }

    /// Gets the effective sort handle for an entity.
    ///
    /// If the entity is in the table, returns its sort handle.
    /// Otherwise, returns the entity's own handle as the sort handle.
    pub fn effective_sort_handle(&self, entity_handle: Handle) -> Handle {
        self.get_sort_handle(entity_handle).unwrap_or(entity_handle)
    }
}

impl Default for SortEntitiesTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sortentitiestable_creation() {
        let table = SortEntitiesTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_for_block() {
        let block_handle = Handle::new(0x1F);
        let table = SortEntitiesTable::for_block(block_handle);
        assert_eq!(table.block_owner_handle, block_handle);
    }

    #[test]
    fn test_add_entry() {
        let mut table = SortEntitiesTable::new();
        let entity = Handle::new(0x100);
        let sort = Handle::new(0x50);

        let result = table.add_entry(entity, sort);
        assert!(result.is_none());
        assert_eq!(table.len(), 1);
        assert!(table.contains(entity));
    }

    #[test]
    fn test_add_entry_update() {
        let mut table = SortEntitiesTable::new();
        let entity = Handle::new(0x100);
        let sort1 = Handle::new(0x50);
        let sort2 = Handle::new(0x75);

        table.add_entry(entity, sort1);
        let result = table.add_entry(entity, sort2);

        assert_eq!(result, Some(sort1));
        assert_eq!(table.len(), 1);
        assert_eq!(table.get_sort_handle(entity), Some(sort2));
    }

    #[test]
    fn test_remove_entry() {
        let mut table = SortEntitiesTable::new();
        let entity = Handle::new(0x100);
        let sort = Handle::new(0x50);

        table.add_entry(entity, sort);
        let result = table.remove_entry(entity);

        assert!(result.is_some());
        assert!(table.is_empty());
        assert!(!table.contains(entity));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut table = SortEntitiesTable::new();
        let result = table.remove_entry(Handle::new(0x100));
        assert!(result.is_none());
    }

    #[test]
    fn test_get_sort_handle() {
        let mut table = SortEntitiesTable::new();
        let entity = Handle::new(0x100);
        let sort = Handle::new(0x50);

        table.add_entry(entity, sort);

        assert_eq!(table.get_sort_handle(entity), Some(sort));
        assert_eq!(table.get_sort_handle(Handle::new(0x999)), None);
    }

    #[test]
    fn test_contains() {
        let mut table = SortEntitiesTable::new();
        let entity = Handle::new(0x100);

        assert!(!table.contains(entity));
        table.add_entry(entity, Handle::new(0x50));
        assert!(table.contains(entity));
    }

    #[test]
    fn test_clear() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x51));

        assert_eq!(table.len(), 2);
        table.clear();
        assert!(table.is_empty());
    }

    #[test]
    fn test_sorted_entries() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x30));
        table.add_entry(Handle::new(0x101), Handle::new(0x10));
        table.add_entry(Handle::new(0x102), Handle::new(0x20));

        let sorted = table.sorted_entries();
        assert_eq!(sorted[0].entity_handle, Handle::new(0x101));
        assert_eq!(sorted[1].entity_handle, Handle::new(0x102));
        assert_eq!(sorted[2].entity_handle, Handle::new(0x100));
    }

    fn block_table() -> (SortEntitiesTable, BlockRecord) {
        let mut block = BlockRecord::new("Test");
        block.handle = Handle::new(0x1F);
        block.entity_handles = vec![Handle::new(0x100), Handle::new(0x101), Handle::new(0x102)];
        (SortEntitiesTable::for_block(block.handle), block)
    }

    #[test]
    fn test_send_to_back() {
        let (mut table, block) = block_table();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x60));

        let handle = table.send_to_back(Handle::new(0x102), &block).unwrap();

        assert!(handle.value() < table.effective_sort_handle(Handle::new(0x100)).value());
        assert!(table.contains(Handle::new(0x102)));
    }

    #[test]
    fn test_bring_to_front() {
        let (mut table, block) = block_table();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x60));

        let handle = table.bring_to_front(Handle::new(0x102), &block).unwrap();

        assert!(handle.value() > 0x60);
        assert!(table.contains(Handle::new(0x102)));
    }

    #[test]
    fn test_move_above() {
        let (mut table, block) = block_table();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));

        let handle = table.move_above(Handle::new(0x101), Handle::new(0x100), &block);

        assert!(handle.is_some());
        assert!(handle.unwrap().value() > 0x50);
    }

    #[test]
    fn test_move_below() {
        let (mut table, block) = block_table();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));

        let handle = table.move_below(Handle::new(0x101), Handle::new(0x100), &block);

        assert!(handle.is_some());
        assert!(handle.unwrap().value() < table.effective_sort_handle(Handle::new(0x100)).value());
    }

    #[test]
    fn test_move_above_nonexistent() {
        let (mut table, block) = block_table();
        let result = table.move_above(Handle::new(0x101), Handle::new(0x200), &block);
        assert!(result.is_none());
    }

    #[test]
    fn test_effective_sort_handle() {
        let mut table = SortEntitiesTable::new();
        let entity1 = Handle::new(0x100);
        let sort1 = Handle::new(0x50);
        table.add_entry(entity1, sort1);

        // Entity in table: use sort handle
        assert_eq!(table.effective_sort_handle(entity1), sort1);

        // Entity not in table: use entity handle
        let entity2 = Handle::new(0x200);
        assert_eq!(table.effective_sort_handle(entity2), entity2);
    }

    #[test]
    fn test_entries_iterator() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x51));

        let count = table.entries().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_entries_mut_iterator() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));

        for entry in table.entries_mut() {
            entry.sort_handle = Handle::new(0x99);
        }

        assert_eq!(
            table.get_sort_handle(Handle::new(0x100)),
            Some(Handle::new(0x99))
        );
    }

    #[test]
    fn test_rebuild_map() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x51));

        // Clear and rebuild
        table.entry_map.clear();
        assert!(!table.contains(Handle::new(0x100)));

        table.rebuild_map();
        assert!(table.contains(Handle::new(0x100)));
        assert!(table.contains(Handle::new(0x101)));
    }

    #[test]
    fn test_default() {
        let table = SortEntitiesTable::default();
        assert!(table.is_empty());
    }

    #[test]
    fn test_sortentsentry_creation() {
        let entry = SortEntsEntry::new(Handle::new(0x100), Handle::new(0x50));
        assert_eq!(entry.entity_handle, Handle::new(0x100));
        assert_eq!(entry.sort_handle, Handle::new(0x50));
    }

    #[test]
    fn test_remove_with_swap() {
        let mut table = SortEntitiesTable::new();
        table.add_entry(Handle::new(0x100), Handle::new(0x50));
        table.add_entry(Handle::new(0x101), Handle::new(0x51));
        table.add_entry(Handle::new(0x102), Handle::new(0x52));

        // Remove middle - should swap with last
        table.remove_entry(Handle::new(0x100));

        assert_eq!(table.len(), 2);
        assert!(table.contains(Handle::new(0x101)));
        assert!(table.contains(Handle::new(0x102)));
        assert!(!table.contains(Handle::new(0x100)));
    }

    #[test]
    fn test_constants() {
        assert_eq!(SortEntitiesTable::OBJECT_NAME, "SORTENTSTABLE");
        assert_eq!(SortEntitiesTable::SUBCLASS_MARKER, "AcDbSortentsTable");
        assert_eq!(SortEntitiesTable::DICTIONARY_KEY, "ACAD_SORTENTS");
    }
}
