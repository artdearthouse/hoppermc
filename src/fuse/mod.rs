//! FUSE filesystem implementation for Minecraft region files.
//!
//! Provides a virtual filesystem that serves procedurally generated
//! Minecraft world data in the Anvil format.

mod inode;

use std::sync::Arc;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;

use crate::chunk::ChunkProvider;
use crate::region::{self, Header, RegionPos, SECTOR_SIZE, HEADER_SIZE};
use crate::storage::ChunkStorage;

use inode::InodeMap;

/// TTL for cached file attributes.
const TTL: Duration = Duration::from_secs(1);

/// Virtual file size for region files (10 MB).
const VIRTUAL_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// FUSE filesystem for Minecraft Anvil regions.
pub struct AnvilFS<S: ChunkStorage> {
    inodes: InodeMap,
    chunks: ChunkProvider<S>,
}

impl<S: ChunkStorage + 'static> AnvilFS<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            inodes: InodeMap::new(),
            chunks: ChunkProvider::new(storage),
        }
    }

    fn root_attr() -> FileAttr {
        FileAttr {
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
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    fn file_attr(ino: u64) -> FileAttr {
        FileAttr {
            ino,
            size: VIRTUAL_FILE_SIZE,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: FileType::RegularFile,
            perm: 0o644,
            nlink: 1,
            uid: 1000,
            gid: 1000,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }

    /// Read data from a virtual region file.
    fn read_region(&self, region: RegionPos, offset: usize, size: usize) -> Vec<u8> {
        let mut result = vec![0u8; size];
        let end = offset + size;

        // Zone A: Header (0 - HEADER_SIZE)
        if offset < HEADER_SIZE {
            let header = Header::generate();
            let copy_start = offset;
            let copy_end = std::cmp::min(end, HEADER_SIZE);
            let copy_len = copy_end - copy_start;
            result[..copy_len].copy_from_slice(&header[copy_start..copy_end]);
        }

        // Zone B: Chunk data (HEADER_SIZE+)
        if end > HEADER_SIZE {
            let data_start = std::cmp::max(offset, HEADER_SIZE);
            let first_chunk = (data_start - HEADER_SIZE) / SECTOR_SIZE;
            let last_chunk = (end - HEADER_SIZE - 1) / SECTOR_SIZE;

            for chunk_idx in first_chunk..=last_chunk {
                if chunk_idx >= 1024 {
                    break;
                }

                let chunk_file_start = HEADER_SIZE + chunk_idx * SECTOR_SIZE;
                let chunk_file_end = chunk_file_start + SECTOR_SIZE;

                let overlap_start = std::cmp::max(offset, chunk_file_start);
                let overlap_end = std::cmp::min(end, chunk_file_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                // Get chunk world coordinates
                let (local_x, local_z) = region::index_to_local(chunk_idx);
                let (world_x, world_z) = region.local_to_world(local_x, local_z);

                // Get chunk data (from storage or generated)
                let pos = crate::storage::ChunkPos::new(world_x, world_z);
                let blob = self.chunks.get_chunk(pos);

                // Copy relevant portion
                let blob_start = overlap_start - chunk_file_start;
                let blob_end = overlap_end - chunk_file_start;
                let result_start = overlap_start - offset;

                for i in blob_start..blob_end {
                    let result_idx = result_start + (i - blob_start);
                    if result_idx < size && i < blob.len() {
                        result[result_idx] = blob[i];
                    }
                }
            }
        }

        result
    }
}

impl<S: ChunkStorage + 'static> Filesystem for AnvilFS<S> {
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        if ino == 1 {
            reply.attr(&TTL, &Self::root_attr());
        } else {
            reply.attr(&TTL, &Self::file_attr(ino));
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let filename = name.to_str().unwrap_or("");

        if let Some(region) = RegionPos::from_filename(filename) {
            let ino = self.inodes.get_or_create(region);
            reply.entry(&TTL, &Self::file_attr(ino), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino == 1 {
            if offset == 0 {
                let _ = reply.add(1, 0, FileType::Directory, ".");
                let _ = reply.add(1, 1, FileType::Directory, "..");
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        let region = match self.inodes.get(ino) {
            Some(r) => r,
            None => {
                reply.data(&vec![0u8; size as usize]);
                return;
            }
        };

        let data = self.read_region(region, offset as usize, size as usize);
        reply.data(&data);
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, reply: fuser::ReplyOpen) {
        reply.opened(0, 0);
    }

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
        // TODO: Parse incoming chunk data and save to storage
        reply.written(data.len() as u32);
    }

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

    fn flush(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }

    fn fsync(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: fuser::ReplyEmpty,
    ) {
        reply.ok();
    }
}
