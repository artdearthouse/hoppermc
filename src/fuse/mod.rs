use fuser::{FileAttr, FileType, Filesystem, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

use std::io::Write;
use crate::region;

use std::sync::Arc;
use crate::generator::WorldGenerator;

// Minecraft Understands only zlib (gzip, nocomp, custom) compression
// but it is much easier to use just zlib (no futher configuration we need)
use flate2::write::ZlibEncoder;
use flate2::Compression;

pub struct McFUSE {
    pub generator: Arc<dyn WorldGenerator>,
}



const DIR_ATTR_TEMPLATE: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 0, gid: 0, rdev: 0, blksize: 512, flags: 0, // uid/gid 0 is ok, we will handle it in code for portability
};

const FILE_ATTR_TEMPLATE: FileAttr = FileAttr {
    ino: 2,
    size: 8192 + (32 * 32 * 64 * 4096), // Header + Data
    blocks: 8, // Non-zero blocks count to show it exists
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 0, gid: 0, rdev: 0, blksize: 512, flags: 0,
};

impl Filesystem for McFUSE {
    // 1. GETATTR (File attributes)
    fn getattr(&mut self, req: &Request, ino: u64, _fh: Option<u64>, reply: fuser::ReplyAttr) {
        match ino {
            1 => { // Directory
                let mut attr = DIR_ATTR_TEMPLATE;
                attr.uid = req.uid(); attr.gid = req.gid();
                reply.attr(&Duration::from_secs(1), &attr);
            },
            2 => { // Our file r.0.0.mca
                let mut attr = FILE_ATTR_TEMPLATE;
                attr.uid = req.uid(); attr.gid = req.gid();
                reply.attr(&Duration::from_secs(1), &attr);
            },
            _ => reply.error(ENOENT),
        }
    }

    // 1.5 ACCESS (Check permissions)
    fn access(&mut self, _req: &Request, ino: u64, _mask: i32, reply: fuser::ReplyEmpty) {
        // We allow everything for everyone (POC)
        if ino == 1 || ino == 2 {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    // 2. LOOKUP (Name search: "What is the inode for r.0.0.mca?")
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
        if parent == 1 && name.to_str() == Some("r.0.0.mca") {
            let mut attr = FILE_ATTR_TEMPLATE;
            attr.uid = req.uid(); attr.gid = req.gid();
            // Generation = 0 (file version), TTL = 1 sec
            reply.entry(&Duration::from_secs(1), &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    // 3. READDIR (LS: "What is inside the folder?")
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: fuser::ReplyDirectory) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        // offset - is the cursor. FUSE can read the directory in chunks.
        // We return: (inode, type, name).
        // Important: offset increases by 1 for each subsequent entry.
        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "r.0.0.mca"),
        ];

        for (i, entry) in entries.into_iter().enumerate() {
            // i + 1, because offset 0 implies "start", and the next entry will be 1, 2, 3...
            if i as i64 >= offset {
                // add returns true if the buffer is full.
                if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                    break;
                }
            }
        }
        reply.ok();
    }

    // 4. WRITE (Write into void)
    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        if ino == 2 {
            // "Honestly" say that we wrote as many bytes as sent
            println!("Writing {} dummy bytes to inode {}", data.len(), ino);
            reply.written(data.len() as u32);
        } else {
            reply.error(ENOENT);
        }
    }

    // 5. READ (The core logic)
    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        if ino != 2 {
            reply.data(&[]);
            return;
        }

        let offset = offset as u64;
        let size = size as usize;
        let mut response_data = Vec::with_capacity(size);

        // --- 1. HEADER GENERATION (0..8192) ---
        // If the request overlaps the header
        if offset < 8192 {
            let mut header = vec![0u8; 8192];
            for i in 0..1024 {
                let rel_x = i % 32;
                let rel_z = i / 32;
                
                // Calculate where the chunk lies using our Sparse formula
                let chunk_offset = region::get_chunk_file_offset(rel_x, rel_z);
                let sector_id = (chunk_offset / 4096) as u32;
                let sector_count = region::SECTORS_PER_CHUNK as u8;

                // Minecraft stores: [Offset:3 bytes][Count:1 byte] (Big Endian)
                let loc_idx = (i as usize) * 4;
                header[loc_idx] = ((sector_id >> 16) & 0xFF) as u8;
                header[loc_idx + 1] = ((sector_id >> 8) & 0xFF) as u8;
                header[loc_idx + 2] = (sector_id & 0xFF) as u8;
                header[loc_idx + 3] = sector_count;
            }
            
            // Copy the requested part of the header into the response
            let start_in_header = offset as usize;
            let end_in_header = std::cmp::min(start_in_header + size, 8192);
            if start_in_header < 8192 {
                response_data.extend_from_slice(&header[start_in_header..end_in_header]);
            }
        }

        // --- 2. CHUNK DATA GENERATION (8192+) ---
        // If we need to fill the rest of the buffer with chunk data
        // --- 2. CHUNK DATA GENERATION (8192+) ---
        // Loop until we filled the buffer or confirmed we are out of bounds
        while response_data.len() < size {
            let current_len = response_data.len();
            let data_read_offset = offset + current_len as u64;
            let needed = size - current_len;

            // Determine which chunk we hit
            if let Some((rel_x, rel_z)) = region::get_chunk_coords_from_offset(data_read_offset) {
                // Generate chunk!
                // Note: In a real system, we should cache this, but for now we regenerate.
                // Because we use deterministic generation, it is safe.
                if let Ok(nbt_data) = self.generator.generate_chunk(rel_x, rel_z) {
                    // Compress (Zlib)
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    if encoder.write_all(&nbt_data).is_ok() {
                        if let Ok(compressed) = encoder.finish() {
                            
                            // Form the chunk "Packet": [Length: 4][Type: 1][Data...]
                            let total_len = (compressed.len() + 1) as u32; // +1 byte for Type
                            let mut chunk_blob = Vec::new();
                            chunk_blob.extend_from_slice(&total_len.to_be_bytes()); // Big Endian Length
                            chunk_blob.push(2); // Type 2 = Zlib
                            chunk_blob.extend_from_slice(&compressed);

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
                                    // We are reading past the actual data of this chunk (Sparse Void)
                                    // Can we skip fast?
                                    // The chunk allocates 256KB (SECTORS_PER_CHUNK * 4096). 
                                    // We are in the "Padding" zone of this chunk.
                                    // We should fill zeros until end of this chunk or end of request.
                                    
                                    let chunk_end_offset = chunk_start_file_offset + (region::SECTORS_PER_CHUNK as u64 * 4096);
                                    let zeros_available = chunk_end_offset.saturating_sub(data_read_offset);
                                    let zeros_to_give = std::cmp::min(zeros_available as usize, needed);
                                    
                                    // Efficient zero filling
                                    response_data.resize(current_len + zeros_to_give, 0);
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            
            // If we are here, we failed to map to a chunk (EOF or Error) or Generation Failed
            // Stop loop to avoid infinite loop
            break;
        }
        
        // Pad with zeros if something is missing (Sparse emptiness)
        if response_data.len() < size {
            response_data.resize(size, 0);
        }

        reply.data(&response_data);
    }
}