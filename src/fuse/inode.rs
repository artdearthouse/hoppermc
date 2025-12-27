
pub const REGION_INODE_START: u64 = 0x8000_0000_0000_0000;

pub fn is_region_inode(ino: u64) -> bool {
    (ino & REGION_INODE_START) != 0
}

pub fn pack(x: i32, z: i32) -> u64 {
    let u_x = (x as u32) as u64;
    let u_z = (z as u32) as u64;
    REGION_INODE_START | (u_x << 32) | u_z
}

pub fn unpack(ino: u64) -> Option<(i32, i32)> {
    if !is_region_inode(ino) {
        return None;
    }
    let val = ino & !REGION_INODE_START;
    let x = (val >> 32) as u32 as i32;
    let z = (val & 0xFFFFFFFF) as u32 as i32;
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
            (i32::MAX, i32::MIN),
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
    fn test_system_inode() {
        assert!(!is_region_inode(1));
        assert!(!is_region_inode(2));
        assert_eq!(unpack(1), None);
    }
}
