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