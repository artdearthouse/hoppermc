use std::sync::Arc;
use crate::generator::WorldGenerator;
use crate::region;

pub struct VirtualFile {
    pub generator: Arc<dyn WorldGenerator>,
}

impl VirtualFile {
    pub fn new(generator: Arc<dyn WorldGenerator>) -> Self {
        Self { generator }
    }

    pub fn read_at(&self, offset: u64, size: usize) -> Vec<u8> {
        let mut response_data = Vec::with_capacity(size);

        // --- 1. HEADER GENERATION (0..8192) ---
        // If the request overlaps the header
        if offset < region::HEADER_BYTES {
            let header = region::generate_header();
            
            // Copy the requested part of the header into the response
            let start_in_header = offset as usize;
            let end_in_header = std::cmp::min(start_in_header + size, region::HEADER_BYTES as usize);
            if start_in_header < region::HEADER_BYTES as usize {
                response_data.extend_from_slice(&header[start_in_header..end_in_header]);
            }
        }

        // --- 2. CHUNK DATA GENERATION (8192+) ---
        // Loop until we filled the buffer or confirmed we are out of bounds
        while response_data.len() < size {
            let current_len = response_data.len();
            let data_read_offset = offset + current_len as u64;
            let needed = size - current_len;

            // Determine which chunk we hit
            if let Some((rel_x, rel_z)) = region::get_chunk_coords_from_offset(data_read_offset) {
                // Generate chunk!
                if let Ok(nbt_data) = self.generator.generate_chunk(rel_x, rel_z) {
                    // Use helper to compress and wrap
                    if let Some(chunk_blob) = region::compress_and_wrap_chunk(&nbt_data) {
                        let chunk_start_file_offset = region::get_chunk_file_offset(rel_x, rel_z);
                        
                        // Which part of this blob do we need?
                        if data_read_offset >= chunk_start_file_offset {
                            let local_offset = (data_read_offset - chunk_start_file_offset) as usize;
                            
                            if local_offset < chunk_blob.len() {
                                let available = chunk_blob.len() - local_offset;
                                let to_copy = std::cmp::min(available, needed);
                                response_data.extend_from_slice(&chunk_blob[local_offset..local_offset + to_copy]);
                                continue; // We made progress
                            } else {
                                // sparse filling
                                let chunk_end_offset = chunk_start_file_offset + (region::SECTORS_PER_CHUNK as u64 * region::SECTOR_BYTES);
                                let zeros_available = chunk_end_offset.saturating_sub(data_read_offset);
                                let zeros_to_give = std::cmp::min(zeros_available as usize, needed);
                                
                                response_data.resize(current_len + zeros_to_give, 0);
                                continue;
                            }
                        }
                    }
                }
            }
            
            // If we are here, we failed to map to a chunk (EOF or Error) or Generation Failed
            break;
        }
        
        // Pad with zeros if something is missing
        if response_data.len() < size {
            response_data.resize(size, 0);
        }

        response_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    struct MockGenerator;
    impl WorldGenerator for MockGenerator {
        fn generate_chunk(&self, _x: i32, _z: i32) -> Result<Vec<u8>> {
            // Return dummy NBT data
            Ok(vec![1, 2, 3, 4])
        }
    }

    #[test]
    fn test_virtual_file_read_header() {
        let generator = Arc::new(MockGenerator);
        let vf = VirtualFile::new(generator);

        // Read first 10 bytes of header
        let data = vf.read_at(0, 10);
        assert_eq!(data.len(), 10);
    }

    #[test]
    fn test_virtual_file_read_chunk_offset() {
        let generator = Arc::new(MockGenerator);
        let vf = VirtualFile::new(generator);

        // Calculate offset for chunk 0,0
        // Header is 8192 bytes
        let chunk_offset = region::get_chunk_file_offset(0, 0); 
        
        // Read 5 bytes from there
        let data = vf.read_at(chunk_offset, 5);
        assert_eq!(data.len(), 5);
        
        // The first 4 bytes are length (big endian). 
        // Our mock returns 4 bytes [1,2,3,4]. Compressed it will be larger.
        // But we can check it's not all zeros.
        assert_ne!(data, vec![0, 0, 0, 0, 0]);
    }
}
