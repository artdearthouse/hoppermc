use clap::Parser;
use std::path::PathBuf;

use hoppermc_fs::McFUSE;
use hoppermc_gen::flat::FlatGenerator;
use hoppermc_gen::vanilla::VanillaWorldGenerator;
use hoppermc_gen::WorldGenerator;
use hoppermc_fs::virtual_file::VirtualFile;

#[derive(Parser)]
#[command(name = "hoppermc", about = "FUSE-based virtual filesystem for Minecraft with Storage Backends")]
pub struct Args {
    #[arg(short, long, default_value = "/mnt/region")]
    pub mountpoint: PathBuf,
    
    /// World generator: "flat" or "vanilla"
    #[arg(short, long, default_value = "flat")]
    pub generator: String,
    
    /// World seed (for vanilla generator)
    #[arg(short, long, default_value = "0")]
    pub seed: u64,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    
    // Default URL using hoppermc user/pass
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@db:5432/hoppermc".to_string());

    println!("Connecting to storage at {}...", database_url);
    
    use hoppermc_storage::{postgres::PostgresStorage, StorageMode, ChunkStorage};
    
    // Retry loop for DB connection
    let mut storage_backend = None;
    for i in 0..30 {
        match PostgresStorage::new(&database_url, StorageMode::Raw).await {
            Ok(s) => {
                storage_backend = Some(s);
                break;
            }
            Err(e) => {
                eprintln!("Failed to connect to storage: {}. Retrying {}/30 in 2s...", e, i + 1);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    let storage_backend = storage_backend.expect("FATAL: Could not connect to storage after 30 retries.");
    
    let storage: std::sync::Arc<dyn ChunkStorage> = std::sync::Arc::new(storage_backend);

    use fuser::MountOption;
    let options = vec![MountOption::AllowOther, MountOption::RW];

    use std::sync::Arc;

    // Select generator based on CLI args
    let generator: Arc<dyn WorldGenerator> = match args.generator.as_str() {
        "vanilla" => {
            println!("Using Pumpkin VanillaGenerator with seed: {}", args.seed);
            Arc::new(VanillaWorldGenerator::new(args.seed))
        },
        "flat" | _ => {
            println!("Using FlatGenerator");
            Arc::new(FlatGenerator)
        },
    };

    let handle = tokio::runtime::Handle::current();
    let virtual_file = VirtualFile::new(generator, storage, handle);
    let fs = McFUSE { virtual_file };

    println!("Mounting HopperMC FUSE to {:?} (Background)", args.mountpoint);
    
    let _session = fuser::spawn_mount2(fs, &args.mountpoint, &options).unwrap();

    println!("Mounted successfully! Press Ctrl+C to unmount");
    
    tokio::signal::ctrl_c().await.expect("failed to install CTRL+C signal handler");
}