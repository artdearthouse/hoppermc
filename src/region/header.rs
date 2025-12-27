//! Region file header generation.
//!
//! The header consists of two tables:
//! - Location table: where each chunk is stored
//! - Timestamp table: when each chunk was last saved

use super::{SECTOR_SIZE, HEADER_SIZE};

/// MCA file header generator.
///
/// Generates the 8KB header (location table + timestamp table)
/// for virtual region files.
pub struct Header;

impl Header {
    /// Generate complete header (8192 bytes).
    pub fn generate() -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];

        // Location table (first 4096 bytes)
        // Each entry: 3 bytes offset + 1 byte sector count
        for i in 0..1024u32 {
            // Each chunk starts at sector (2 + i)
            // Sector 0-1 are the header itself
            let sector_offset = 2 + i;
            let sector_count: u8 = 1;

            let entry_offset = (i as usize) * 4;
            header[entry_offset] = ((sector_offset >> 16) & 0xFF) as u8;
            header[entry_offset + 1] = ((sector_offset >> 8) & 0xFF) as u8;
            header[entry_offset + 2] = (sector_offset & 0xFF) as u8;
            header[entry_offset + 3] = sector_count;
        }

        // Timestamp table (second 4096 bytes) - all zeros
        // Already initialized to 0

        header
    }

    /// Get a slice of the header for a specific byte range.
    pub fn get_range(offset: usize, size: usize) -> Vec<u8> {
        let header = Self::generate();
        let end = std::cmp::min(offset + size, HEADER_SIZE);
        if offset >= HEADER_SIZE {
            vec![0u8; size]
        } else {
            let mut result = header[offset..end].to_vec();
            // Pad with zeros if request extends beyond header
            if result.len() < size {
                result.resize(size, 0);
            }
            result
        }
    }

    /// Calculate sector offset for a chunk index.
    #[inline]
    pub fn chunk_sector(chunk_index: usize) -> u32 {
        2 + chunk_index as u32
    }

    /// Calculate file offset for a chunk index.
    #[inline]
    pub fn chunk_offset(chunk_index: usize) -> usize {
        Self::chunk_sector(chunk_index) as usize * SECTOR_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        let header = Header::generate();
        assert_eq!(header.len(), 8192);
    }

    #[test]
    fn test_first_chunk_location() {
        let header = Header::generate();
        // First chunk (index 0) should be at sector 2
        assert_eq!(header[0], 0); // high byte
        assert_eq!(header[1], 0); // mid byte
        assert_eq!(header[2], 2); // low byte = sector 2
        assert_eq!(header[3], 1); // size = 1 sector
    }

    #[test]
    fn test_chunk_offset() {
        // Chunk 0 at sector 2 = byte 8192
        assert_eq!(Header::chunk_offset(0), 8192);
        // Chunk 1 at sector 3 = byte 12288
        assert_eq!(Header::chunk_offset(1), 12288);
    }
}
