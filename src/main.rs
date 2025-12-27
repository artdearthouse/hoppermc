use clap::Parser;
use std::path::PathBuf;

mod fuse;

#[derive(Parser)]
#[command(name = "mc-anvil-db", about = "FUSE-based virtual filesystem for Minecraft with Storage Backends")]
pub struct Args {
    #[arg(short, long, default_value = "/mnt/mc")]
    pub mountpoint: PathBuf,
}