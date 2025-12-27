// --- IMPORTS ---
// We attach the module we just created
mod nbt_structs;

use crate::nbt_structs::*;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT; // "Error No Entry" - standard Linux error for "File not found"
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};
use flate2::write::ZlibEncoder;
use flate2::Compression;

// --- CONSTANTS ---
// How long the Kernel should cache file attributes.
// 1 second is good for testing. In production, you'd want higher.
const TTL: Duration = Duration::from_secs(1);

// Region files (.mca) have a specific structure:
// Header 1 (Locations): 4096 bytes
// Header 2 (Timestamps): 4096 bytes
// Data starts at byte 8192.
const HEADER_SIZE: u64 = 8192;
const CHUNK_PADDING: u64 = 4096; // We virtually align every chunk to 4KB

// --- THE DRIVER STRUCT ---
struct AnvilFS {
    // Maps inode -> (region_x, region_z)
    // We use Mutex because FUSE callbacks need &mut self
    inode_map: Mutex<HashMap<u64, (i32, i32)>>,
    // Next available inode (starts at 2, as 1 is root)
    next_inode: Mutex<u64>,
}

impl AnvilFS {
    fn new() -> Self {
        AnvilFS {
            inode_map: Mutex::new(HashMap::new()),
            next_inode: Mutex::new(2), // 1 is reserved for root
        }
    }

    // Parse "r.X.Z.mca" -> Some((X, Z))
    fn parse_region_name(name: &str) -> Option<(i32, i32)> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 4 && parts[0] == "r" && parts[3] == "mca" {
            let x = parts[1].parse::<i32>().ok()?;
            let z = parts[2].parse::<i32>().ok()?;
            Some((x, z))
        } else {
            None
        }
    }

    // Get or create inode for a region
    fn get_or_create_inode(&self, region_x: i32, region_z: i32) -> u64 {
        let mut map = self.inode_map.lock().unwrap();

        // Check if we already have an inode for this region
        for (&ino, &(rx, rz)) in map.iter() {
            if rx == region_x && rz == region_z {
                return ino;
            }
        }

        // Create new inode
        let mut next = self.next_inode.lock().unwrap();
        let ino = *next;
        *next += 1;
        map.insert(ino, (region_x, region_z));
        ino
    }

    // --- THE CORE LOGIC: Procedural Generation ---
    // This function runs in RAM. It creates the NBT structure -> Bytes -> Zlib -> Chunk Blob
    fn generate_chunk_bytes(&self, chunk_x: i32, chunk_z: i32) -> Vec<u8> {
        let mut sections = Vec::new();

        // Generate sections from Y=-4 to Y=19 (Total height: 384 blocks)
        for section_y in -4..20 {
            // Logic: Bottom section (y=-4) is Bedrock. Everything else is Air.
            let block_name = if section_y == -4 {
                "minecraft:dirt"
            } else {
                "minecraft:air"
            };

            // Create the palette.
            // If it's bedrock, the palette is ["minecraft:bedrock"].
            // If it's air, the palette is ["minecraft:air"].
            let palette = vec![BlockState {
                name: block_name.to_string(),
            }];

            sections.push(Section {
                y: section_y as i8,
                block_states: BlockStates { palette },
                biomes: Biomes {
                    // Biomes are mandatory in 1.21. We set everything to Plains.
                    palette: vec!["minecraft:plains".to_string()],
                },
            });
        }

        // Assemble the Chunk
        let chunk = ChunkData {
            data_version: 3955, // 1.21.1 Version ID
            x_pos: chunk_x,
            z_pos: chunk_z,
            y_pos: -4, // Lowest section Y (-4 * 16 = -64)
            status: "minecraft:full".to_string(),
            last_update: 0,
            inhabited_time: 0,
            is_light_on: 1, // Light has been calculated
            sections,
        };

        // 1. Serialize struct to NBT bytes
        let nbt_data = fastnbt::to_bytes(&chunk).unwrap();

        // 2. Compress using Zlib (required by Minecraft)
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&nbt_data).unwrap();
        let compressed_data = encoder.finish().unwrap();

        // 3. Wrap in MCA format: [Length (4 bytes)] + [CompressionType (1 byte)] + [Data]
        let mut final_blob = Vec::new();
        let total_len = (compressed_data.len() + 1) as u32; // +1 for the compression byte
        final_blob.extend_from_slice(&total_len.to_be_bytes()); // Big Endian!
        final_blob.push(2); // Type 2 = Zlib
        final_blob.extend_from_slice(&compressed_data);

        final_blob
    }
}

// --- FUSE IMPLEMENTATION ---
impl Filesystem for AnvilFS {
    // 1. GETATTR (What is this file?)
    // The OS asks: "I see file with ID 2. How big is it? What are the permissions?"
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let ts = UNIX_EPOCH;
        
        // Inode 1 is ALWAYS the root directory in FUSE
        if ino == 1 {
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: ts, mtime: ts, ctime: ts, crtime: ts,
                kind: FileType::Directory, // It's a folder
                perm: 0o755,               // rwxr-xr-x
                nlink: 2,
                uid: 1000, gid: 1000, rdev: 0, flags: 0, blksize: 512,
            };
            reply.attr(&TTL, &attr);
        } else {
            // Any other Inode is considered a Region File
            let attr = FileAttr {
                ino: ino,
                size: 10 * 1024 * 1024, // Fake size: 10MB. 
                                        // It must be large enough so Java thinks it can seek inside.
                blocks: 1,
                atime: ts, mtime: ts, ctime: ts, crtime: ts,
                kind: FileType::RegularFile, // It's a file
                perm: 0o644,                 // rw-r--r--
                nlink: 1,
                uid: 1000, gid: 1000, rdev: 0, flags: 0, blksize: 512,
            };
            reply.attr(&TTL, &attr);
        }
    }

    // 2. LOOKUP (Does this file exist?)
    // The OS asks: "Does 'r.0.0.mca' exist in folder 1?"
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // We only allow lookups in the root (parent == 1)
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let filename = name.to_str().unwrap_or("");

        // Parse region filename to get coordinates
        if let Some((region_x, region_z)) = Self::parse_region_name(filename) {
            // Get or create a unique inode for this region
            let ino = self.get_or_create_inode(region_x, region_z);

            let ts = UNIX_EPOCH;
            let attr = FileAttr {
                ino,
                size: 10 * 1024 * 1024,
                blocks: 1,
                atime: ts, mtime: ts, ctime: ts, crtime: ts,
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1, uid: 1000, gid: 1000, rdev: 0, flags: 0, blksize: 512,
            };
            reply.entry(&TTL, &attr, 0);
        } else {
            // Not a valid region file
            reply.error(ENOENT);
        }
    }

    // 3. READDIR (List files in folder)
    // The OS asks: "Show me the list of files"
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino == 1 {
            if offset == 0 {
                // Standard: current dir (.) and parent (..)
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");
                // We don't list any actual .mca files here.
                // The server knows what it's looking for (e.g. "r.0.0.mca") and will call LOOKUP directly.
                // This keeps `ls` clean.
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    // 4. READ (Give me the bytes!)
    // Handles reads that may span multiple zones (header, timestamps, chunk data)
    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, _flags: i32, _lock: Option<u64>, reply: ReplyData) {
        // Look up region coordinates from inode
        let (region_x, region_z) = {
            let map = self.inode_map.lock().unwrap();
            match map.get(&ino) {
                Some(&coords) => coords,
                None => {
                    reply.data(&vec![0u8; size as usize]);
                    return;
                }
            }
        };

        let offset = offset as usize;
        let size = size as usize;
        let mut result = vec![0u8; size];
        let mut pos = 0usize; // Position in result buffer

        // --- ZONE A: Location Table (bytes 0-4095) ---
        if offset < 4096 && pos < size {
            let zone_start = offset;
            let zone_end = std::cmp::min(offset + size, 4096);
            let bytes_to_copy = zone_end - zone_start;

            // Generate location table
            for i in 0..1024u32 {
                let sector_offset = 2 + i;
                let entry_start = (i as usize) * 4;

                // Only generate entries we need
                if entry_start + 4 > zone_start && entry_start < zone_end {
                    let bytes = [
                        ((sector_offset >> 16) & 0xFF) as u8,
                        ((sector_offset >> 8) & 0xFF) as u8,
                        (sector_offset & 0xFF) as u8,
                        1u8, // sector count
                    ];
                    for (j, &byte) in bytes.iter().enumerate() {
                        let file_pos = entry_start + j;
                        if file_pos >= zone_start && file_pos < zone_end {
                            result[pos + file_pos - zone_start] = byte;
                        }
                    }
                }
            }
            pos += bytes_to_copy;
        }

        // --- ZONE B: Timestamps (bytes 4096-8191) ---
        if offset + size > 4096 && offset < 8192 && pos < size {
            let zone_start = std::cmp::max(offset, 4096);
            let zone_end = std::cmp::min(offset + size, 8192);
            let bytes_to_copy = zone_end - zone_start;
            // Timestamps are zeros, already filled by vec![0u8; size]
            pos += bytes_to_copy;
        }

        // --- ZONE C: Chunk Data (bytes 8192+) ---
        if offset + size > 8192 && pos < size {
            let data_start = std::cmp::max(offset, 8192);
            let data_end = offset + size;

            // Process each chunk that the read touches
            let first_chunk = (data_start - 8192) / CHUNK_PADDING as usize;
            let last_chunk = (data_end - 8192 - 1) / CHUNK_PADDING as usize;

            for chunk_idx in first_chunk..=last_chunk {
                let chunk_file_start = 8192 + chunk_idx * CHUNK_PADDING as usize;
                let chunk_file_end = chunk_file_start + CHUNK_PADDING as usize;

                // Calculate overlap between request and this chunk
                let overlap_start = std::cmp::max(offset, chunk_file_start);
                let overlap_end = std::cmp::min(offset + size, chunk_file_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                // Generate chunk data
                let local_z = (chunk_idx / 32) as i32;
                let local_x = (chunk_idx % 32) as i32;
                let world_chunk_x = region_x * 32 + local_x;
                let world_chunk_z = region_z * 32 + local_z;
                let blob = self.generate_chunk_bytes(world_chunk_x, world_chunk_z);

                // Copy relevant portion
                let blob_start = overlap_start - chunk_file_start;
                let blob_end = overlap_end - chunk_file_start;
                let result_start = overlap_start - offset;

                for i in blob_start..blob_end {
                    let result_idx = result_start + (i - blob_start);
                    if result_idx < size {
                        result[result_idx] = if i < blob.len() { blob[i] } else { 0 };
                    }
                }
            }
        }

        reply.data(&result);
    }

    // 5. OPEN (Open a file handle)
    // Minecraft needs to open files in read-write mode. We allow it.
    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        // Return a dummy file handle (0) with no special flags
        reply.opened(0, 0);
    }

    // 6. WRITE (Accept writes but discard them)
    // For MVP: we pretend to accept writes but don't store anything.
    // This allows Minecraft to "save" without errors.
    fn write(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyWrite,
    ) {
        // Pretend we wrote all the bytes
        reply.written(data.len() as u32);
    }

    // 7. RELEASE (Close file handle)
    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    // 8. FLUSH (Sync data before close)
    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: fuser::ReplyEmpty) {
        reply.ok();
    }

    // 9. FSYNC (Force sync to disk)
    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: fuser::ReplyEmpty) {
        reply.ok();
    }
}

// --- MAIN ENTRY POINT ---
fn main() {
    // 1. Initialize Logger (so we can see what's happening via RUST_LOG=info)
    env_logger::init();

    // 2. Define mount point (Where the folder appears)
    let mountpoint = "/mnt/region";

    // 3. FUSE Options
    let options = vec![
        MountOption::FSName("mc-anvil-db".to_string()),
        MountOption::AutoUnmount, // Clean up on exit
        MountOption::AllowOther,  // REQUIRED for Docker to share the mount
    ];

    println!("Starting FUSE mount at {}...", mountpoint);

    // 4. Start the loop. This blocks forever until the program is killed.
    fuser::mount2(AnvilFS::new(), mountpoint, &options).unwrap();
}