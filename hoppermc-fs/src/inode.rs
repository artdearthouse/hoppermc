
// 24 bits for X and Z. Range +/- 8 million regions.
// Flag at bit 63 (Region) and 62 (Generic)
// Structure:
// Bit 63: Region Flag
// Bit 62: Generic Flag
// Bits 24..47: X (24 bits)
// Bits 0..23: Z (24 bits)

const OFFSET: i32 = 8_000_000;
const MASK: u64 = 0xFFFFFF; // 24 bits

pub const REGION_INODE_START: u64 = 0x8000_0000_0000_0000;
pub const GENERIC_INODE_START: u64 = 0x4000_0000_0000_0000;

pub fn is_region_inode(ino: u64) -> bool {
    (ino & REGION_INODE_START) != 0
}

pub fn is_generic_inode(ino: u64) -> bool {
    (ino & GENERIC_INODE_START) != 0
}

pub fn pack(x: i32, z: i32) -> u64 {
    // Offset to make positive
    let x_enc = (x + OFFSET) as u64 & MASK;
    let z_enc = (z + OFFSET) as u64 & MASK;
    
    REGION_INODE_START | (x_enc << 24) | z_enc
}

// FNV-1a 64-bit hash
fn fnv1a_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x1099511628211);
    }
    hash
}

pub fn pack_generic(name: &str) -> u64 {
    let hash = fnv1a_hash(name);
    // Mask to 62 bits to avoid colliding with flags (top 2 bits)
    GENERIC_INODE_START | (hash & 0x3FFF_FFFF_FFFF_FFFF)
}

pub fn unpack(ino: u64) -> Option<(i32, i32)> {
    if !is_region_inode(ino) {
        return None;
    }
    
    let x_enc = (ino >> 24) & MASK;
    let z_enc = ino & MASK;
    
    let x = (x_enc as i32) - OFFSET;
    let z = (z_enc as i32) - OFFSET;
    
    Some((x, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack() {
        let coords = [
            (0, 0),
            (1, 1),
            (-1, -1),
            (100, -100),
            (7_000_000, -7_000_000), // Within +/- 8M
            (-7_999_999, 7_999_999), 
        ];

        for (x, z) in coords {
            let ino = pack(x, z);
            assert!(is_region_inode(ino));
            let (rx, rz) = unpack(ino).expect("Should unpack");
            assert_eq!(rx, x);
            assert_eq!(rz, z);
        }
    }

    #[test]
    fn test_generic_inodes() {
        let name = "backup.mca";
        let ino = pack_generic(name);
        assert!(is_generic_inode(ino));
        assert!(!is_region_inode(ino));
        
        // Hash stability check (FNV-1a of "backup.mca" should be stable)
        let ino2 = pack_generic(name);
        assert_eq!(ino, ino2);
        
        let name2 = "other.file";
        let ino3 = pack_generic(name2);
        assert_ne!(ino, ino3);
    }
    #[test]
    fn test_system_inode() {
        assert!(!is_region_inode(1));
        assert!(!is_region_inode(2));
        assert_eq!(unpack(1), None);
    }
}
