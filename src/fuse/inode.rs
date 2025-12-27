//! Inode management for FUSE filesystem.
//!
//! Maps inode numbers to region file coordinates.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::region::RegionPos;

/// Maps inode numbers to region positions.
///
/// Inode 1 is reserved for the root directory.
/// Inodes 2+ are dynamically assigned to region files.
pub struct InodeMap {
    to_region: Mutex<HashMap<u64, RegionPos>>,
    to_inode: Mutex<HashMap<RegionPos, u64>>,
    next_inode: Mutex<u64>,
}

impl InodeMap {
    pub fn new() -> Self {
        Self {
            to_region: Mutex::new(HashMap::new()),
            to_inode: Mutex::new(HashMap::new()),
            next_inode: Mutex::new(2), // 1 is root
        }
    }

    /// Get or create an inode for a region.
    pub fn get_or_create(&self, region: RegionPos) -> u64 {
        // Check if already exists
        {
            let map = self.to_inode.lock().unwrap();
            if let Some(&ino) = map.get(&region) {
                return ino;
            }
        }

        // Create new inode
        let mut next = self.next_inode.lock().unwrap();
        let ino = *next;
        *next += 1;

        self.to_region.lock().unwrap().insert(ino, region);
        self.to_inode.lock().unwrap().insert(region, ino);

        ino
    }

    /// Get region for an inode.
    pub fn get(&self, ino: u64) -> Option<RegionPos> {
        self.to_region.lock().unwrap().get(&ino).copied()
    }
}

impl Default for InodeMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_creation() {
        let map = InodeMap::new();

        let r1 = RegionPos::new(0, 0);
        let r2 = RegionPos::new(1, -1);

        let ino1 = map.get_or_create(r1);
        let ino2 = map.get_or_create(r2);

        assert_eq!(ino1, 2);
        assert_eq!(ino2, 3);

        // Same region should return same inode
        assert_eq!(map.get_or_create(r1), ino1);
    }

    #[test]
    fn test_inode_lookup() {
        let map = InodeMap::new();
        let r = RegionPos::new(5, -3);

        let ino = map.get_or_create(r);
        assert_eq!(map.get(ino), Some(r));
        assert_eq!(map.get(999), None);
    }
}
