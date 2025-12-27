//! mc-anvil-db: FUSE filesystem for procedural Minecraft world generation.
//!
//! This program mounts a virtual filesystem that serves Minecraft region files
//! (.mca) with procedurally generated chunk data.

mod chunk;
mod fuse;
mod nbt;
mod region;
mod storage;

use std::sync::Arc;
use fuser::MountOption;

use crate::fuse::AnvilFS;
use crate::storage::MemoryStorage;

fn main() {
    env_logger::init();

    let mountpoint = "/mnt/region";

    let options = vec![
        MountOption::FSName("mc-anvil-db".to_string()),
        MountOption::AutoUnmount,
        MountOption::AllowOther,
    ];

    // Create storage backend
    let storage = Arc::new(MemoryStorage::new());

    // Create filesystem
    let fs = AnvilFS::new(storage);

    println!("Starting mc-anvil-db FUSE mount at {}...", mountpoint);

    fuser::mount2(fs, mountpoint, &options).expect("Failed to mount FUSE filesystem");
}
