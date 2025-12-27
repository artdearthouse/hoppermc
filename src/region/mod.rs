// Sparse Files for Emulationg Real files (so minecraft will see weight of file)

pub const SECTOR_BYTES: u64 = 4096; // minecraft uses 4096 bytes per sector     
pub const HEADER_BYTES: u64 = 8192; // header is 8192 bytes (2 sectors 8kb) 


pub const SECTORS_PER_CHUNK: u64 = 64; // 256kb per chunk


pub fn get_chunk_file_offset(rel_x: i32, rel_z: i32) -> u64 {
    // 32x32 chunks in region. index from 0 to 1023.
    // Formula: x + z * 32
    let index = (rel_x & 31) + (rel_z & 31) * 32; 
    
    // Offset = Header + (Chunk index * Sector size)
    HEADER_BYTES + (index as u64 * SECTORS_PER_CHUNK * SECTOR_BYTES)
}


pub fn get_chunk_coords_from_offset(offset: u64) -> Option<(i32, i32)> {
    if offset < HEADER_BYTES {
        return None; // Header, no chunks here
    }
    let data_offset = offset - HEADER_BYTES;
    let slot_size = SECTORS_PER_CHUNK * SECTOR_BYTES;
    
    let index = data_offset / slot_size;
    if index >= 1024 {
        return None; // Out of bounds
    }
    // Reverse math: x = index % 32, z = index / 32
    let rel_x = (index % 32) as i32;
    let rel_z = (index / 32) as i32;
    
    Some((rel_x, rel_z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_offset_0_0() {
        // 0,0 -> Index 0 -> Offset = Header (8192)
        let offset = get_chunk_file_offset(0, 0);
        assert_eq!(offset, HEADER_BYTES);
    }

    #[test]
    fn test_chunk_offset_31_0() {
        // 31,0 -> Index 31
        let offset = get_chunk_file_offset(31, 0);
        let expected = HEADER_BYTES + (31 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(offset, expected);
    }

    #[test]
    fn test_chunk_offset_0_1() {
        // 0,1 -> Index 32
        let offset = get_chunk_file_offset(0, 1);
        let expected = HEADER_BYTES + (32 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(offset, expected);
    }

    #[test]
    fn test_round_trip() {
        // Test all possible chunks in a region (32x32)
        for z in 0..32 {
            for x in 0..32 {
                let offset = get_chunk_file_offset(x, z);
                
                // Verify we point to the start of a chunk
                let (res_x, res_z) = get_chunk_coords_from_offset(offset).expect("Should find coords");
                assert_eq!(res_x, x, "Mismatch X");
                assert_eq!(res_z, z, "Mismatch Z");

                // Verify we point somewhere inside the chunk too
                let mid_offset = offset + 1234; // Random offset inside
                let (res_x_mid, res_z_mid) = get_chunk_coords_from_offset(mid_offset).expect("Should find coords inside");
                assert_eq!(res_x_mid, x);
                assert_eq!(res_z_mid, z);
            }
        }
    }

    #[test]
    fn test_out_of_bounds() {
        // Before header
        assert_eq!(get_chunk_coords_from_offset(0), None);
        assert_eq!(get_chunk_coords_from_offset(8191), None);

        // Way too far (Index 1024 starts at Header + 1024 * ChunkSize)
        let max_valid = HEADER_BYTES + (1024 * SECTORS_PER_CHUNK * SECTOR_BYTES);
        assert_eq!(get_chunk_coords_from_offset(max_valid), None); // First byte of next region (conceptually)
    }
}