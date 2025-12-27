use fuser::{FileAttr, FileType, Filesystem, Request};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH, SystemTime};
use crate::region;


pub mod virtual_file;
pub mod inode;

use virtual_file::VirtualFile;

pub struct McFUSE {
    pub virtual_file: VirtualFile,
}

// Helper to parse "r.x.z.mca"
fn parse_region_filename(name: &str) -> Option<(i32, i32)> {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    if parts[0] != "r" || parts[3] != "mca" {
        return None;
    }
    let x = parts[1].parse::<i32>().ok()?;
    let z = parts[2].parse::<i32>().ok()?;
    Some((x, z))
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
    ino: 2, // Placeholder
    size: region::HEADER_BYTES + (32 * 32 * region::SECTORS_PER_CHUNK * region::SECTOR_BYTES), // Header + Data
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
            _ => {
                if inode::is_region_inode(ino) {
                    let mut attr = FILE_ATTR_TEMPLATE;
                    attr.ino = ino;
                    attr.uid = req.uid(); attr.gid = req.gid();
                    reply.attr(&Duration::from_secs(1), &attr);
                } else {
                    reply.error(ENOENT);
                }
            }
        }
    }

    // 1.5 ACCESS (Check permissions)
    fn access(&mut self, _req: &Request, ino: u64, _mask: i32, reply: fuser::ReplyEmpty) {
        // We allow everything for everyone (POC)
        if ino == 1 || inode::is_region_inode(ino) {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    // 2. LOOKUP (Name search)
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: fuser::ReplyEntry) {
        if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if let Some((x, z)) = parse_region_filename(name_str) {
            let ino = inode::pack(x, z);
            let mut attr = FILE_ATTR_TEMPLATE;
            attr.ino = ino;
            attr.uid = req.uid(); attr.gid = req.gid();
            // Generation = 0 (file version), TTL = 1 sec
            reply.entry(&Duration::from_secs(1), &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    // 3. READDIR (LS)
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: fuser::ReplyDirectory) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        // offset - is the cursor. FUSE can read the directory in chunks.
        // We return: (inode, type, name).
        // Important: offset increases by 1 for each subsequent entry.
        
        // For now, let's just list 0.0 as a sample, or nothing?
        // Minecraft doesn't rely on readdir to find regions, it knows where they should be.
        // It's mostly for us humans.
        
        let entries = vec![
            (1, FileType::Directory, ".".to_string()),
            (1, FileType::Directory, "..".to_string()),
            (inode::pack(0, 0), FileType::RegularFile, "r.0.0.mca".to_string()),
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
    
    // --- STUBS for Create/Write/etc ---

    // CREATE
    fn create(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: fuser::ReplyCreate,
    ) {
         if parent != 1 {
            reply.error(ENOENT);
            return;
        }

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(ENOENT);
                return;
            }
        };

        if let Some((x, z)) = parse_region_filename(name_str) {
            let ino = inode::pack(x, z);
            let mut attr = FILE_ATTR_TEMPLATE;
            attr.ino = ino;
            attr.uid = req.uid(); attr.gid = req.gid();
            
            // Reply with entry + opened handle (we use 0 as dumb fh)
            reply.created(&Duration::from_secs(1), &attr, 0, 0, 0);
        } else {
            reply.error(libc::EACCES); // Or EPERM? Only allow r.x.z.mca
        }
    }

    // SETATTR
    fn setattr(
        &mut self,
        req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: fuser::ReplyAttr,
    ) {
        if inode::is_region_inode(ino) {
            let mut attr = FILE_ATTR_TEMPLATE;
            attr.ino = ino;
            attr.uid = req.uid(); attr.gid = req.gid();
             // In a real FS, we would update the attributes. Here we just say "Sure!"
             // If size is changed, we might want to log it?
            reply.attr(&Duration::from_secs(1), &attr);
        } else {
             reply.error(ENOENT);
        }
    }

    // UNLINK (Delete)
    fn unlink(&mut self, _req: &Request, parent: u64, _name: &OsStr, reply: fuser::ReplyEmpty) {
        if parent == 1 {
            // "Deleted"
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    // RENAME
    fn rename(
        &mut self,
        _req: &Request,
        parent: u64,
        _name: &OsStr,
        newparent: u64,
        _newname: &OsStr,
        _flags: u32,
        reply: fuser::ReplyEmpty,
    ) {
        if parent == 1 && newparent == 1 {
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }


    // 4. WRITE (Write into void/virtual file)
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
        if inode::is_region_inode(ino) {
            // "Honestly" say that we wrote as many bytes as sent
            // println!("Writing {} dummy bytes to inode {}", data.len(), ino);
            // TODO: Pass to virtual file if we want to handle writes seriously
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
        if let Some((_x, _z)) = inode::unpack(ino) {
             let offset = offset as u64;
             let size = size as usize;
             // Pass x, z to virtual file so it knows what to generate?
             // Currently VirtualFile.read_at doesn't take x,z. 
             // But VirtualFile.read_at logic essentially calculates x,z from offset assuming it is *A* region file.
             // Wait. VirtualFile design assumes it represents ONE file.
             // But now we have MANY files (r.0.0, r.0.1, ...).
             // If I ask for r.1.1.mca, and I read offset 0, VirtualFile::read_at needs to know it is r.1.1.
             
             // CRITICAL FIX: VirtualFile needs to know which region it is simulating?
             // OR: VirtualFile::read_at needs to take (region_x, region_z).
             
             // The current VirtualFile implementation:
             // region::get_chunk_coords_from_offset(data_read_offset) -> (rel_x, rel_z)
             // This is RELATIVE to the region (0..31).
             // AND: self.generator.generate_chunk(rel_x, rel_z)
             // This generator needs ABSOLUTE coordinates if we want distinct content for distinct regions!
             
             // For now, let's just make it work for "any" region (they will all look identical).
             // That is acceptable for the "Stub" phase.
             
             reply.data(&self.virtual_file.read_at(offset, size));
        } else {
            reply.data(&[]);
        }
    }
}