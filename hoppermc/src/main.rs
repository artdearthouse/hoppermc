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
    #[arg(short, long, env = "GENERATOR", default_value = "flat")]
    pub generator: String,
    
    /// World seed (for vanilla generator)
    #[arg(long, env = "SEED", default_value = "0")]
    pub seed: u64,
    
    /// Storage mode: "nostorage", "pg_raw", or "pg_jsonb"
    #[arg(long, env = "STORAGE", default_value = "pg_raw")]
    pub storage: String,

    /// Cache size (number of chunks)
    #[arg(long, env("CACHE_SIZE"), default_value_t = 500)]
    pub cache_size: usize,

    /// Prefetch radius (chunks). 0 = disabled.
    #[arg(long, env("PREFETCH_RADIUS"), default_value_t = 0)]
    pub prefetch_radius: u8,

    /// Auto-benchmark mode: cycle through all configurations
    #[arg(long, env("AUTO_BENCHMARK"), default_value_t = false)]
    pub auto_benchmark: bool,

    /// Duration for each benchmark cycle (seconds)
    #[arg(long, env("BENCHMARK_CYCLE_DURATION"), default_value_t = 60)]
    pub benchmark_cycle_duration: u64,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    
    use hoppermc_storage::{postgres::PostgresStorage, StorageMode, ChunkStorage};
    use std::sync::Arc;
    
    // Initialize storage based on mode
    let storage: Option<Arc<dyn ChunkStorage>> = match args.storage.to_lowercase().as_str() {
        "nostorage" | "none" | "stateless" => {
            println!("Storage mode: NOSTORAGE (stateless, all chunks generated on-the-fly)");
            None
        },
        "pg_raw" | "raw" | "postgres" | "pg_jsonb" | _ => {
            let database_url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@db:5432/hoppermc".to_string());
            
            let mode = if args.storage.to_lowercase() == "pg_jsonb" {
                StorageMode::PgJsonb
            } else {
                StorageMode::PgRaw
            };

            println!("Storage mode: {:?} (PostgreSQL)", mode);
            println!("Connecting to storage at {}...", database_url);
            
            // Retry loop for DB connection
            let mut storage_backend = None;
            for i in 0..30 {
                match PostgresStorage::new(&database_url, mode).await {
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

            let backend = storage_backend.expect("FATAL: Could not connect to storage after 30 retries.");
            Some(Arc::new(backend) as Arc<dyn ChunkStorage>)
        }
    };

    use fuser::MountOption;
    let options = vec![MountOption::AllowOther, MountOption::RW];

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

    // Initialize Benchmark with Config Summary
    use hoppermc_benchmark::BenchmarkMetrics;
    let benchmark = if std::env::var("BENCHMARK").is_ok() {
        println!("BENCHMARK MODE ENABLED 🚀");
        let config_summary = format!(
            "Gen: {} | Seed: {} | Storage: {} | Cache: {} | Prefetch: {}", 
            args.generator, args.seed, args.storage, args.cache_size, args.prefetch_radius
        );
        Some(Arc::new(BenchmarkMetrics::new(config_summary)))
    } else {
        None
    };

    if args.auto_benchmark {
        run_auto_benchmark(args, benchmark).await;
        return;
    }

    let handle = tokio::runtime::Handle::current();
    let virtual_file = Arc::new(VirtualFile::new(generator, storage, handle, benchmark.clone(), args.cache_size, args.prefetch_radius));
    let fs = McFUSE { virtual_file: virtual_file.clone() };

    println!("Mounting HopperMC FUSE to {:?} (Background)", args.mountpoint);
    
    let _session = fuser::spawn_mount2(fs, &args.mountpoint, &options).unwrap();

    println!("Mounted successfully! Waiting for shutdown signal...");
    
    // Handle both SIGINT (Ctrl+C) and SIGTERM (Docker stop)
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => println!("Received SIGINT"),
        _ = sigterm.recv() => println!("Received SIGTERM"),
    }

    // Write Benchmark Report
    if let Some(bench) = benchmark {
        println!("Received shutdown signal, unmounting...");
        
        // Fetch final storage size if storage is enabled
        if let Some(storage) = &virtual_file.storage {
             match storage.get_total_size().await {
                 Ok(size) => bench.record_db_size(size),
                 Err(e) => eprintln!("Failed to fetch storage size for benchmark: {}", e),
             }
        }

        let report = bench.generate_report();
        write_report(report);
    }
}

fn write_report(report: String) {
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if let Err(e) = std::fs::create_dir_all("benchmarks") {
        eprintln!("Failed to create benchmarks directory: {}", e);
    }
    let filename = format!("benchmarks/benchmark-{}.txt", timestamp);
    if let Err(e) = std::fs::write(&filename, &report) {
        eprintln!("Failed to write benchmark report: {}", e);
    } else {
        println!("Benchmark report written to {}", filename);
        println!("{}", report);
    }
}

async fn run_auto_benchmark(args: Args, _main_bench: Option<std::sync::Arc<hoppermc_benchmark::BenchmarkMetrics>>) {
    use hoppermc_storage::{postgres::PostgresStorage, StorageMode, ChunkStorage};
    use hoppermc_gen::flat::FlatGenerator;
    use hoppermc_gen::vanilla::VanillaWorldGenerator;
    use hoppermc_gen::WorldGenerator;
    use hoppermc_benchmark::BenchmarkMetrics;
    use hoppermc_fs::virtual_file::VirtualFile;
    use hoppermc_anvil::get_chunk_file_offset;
    use std::sync::Arc;
    use std::time::Duration;

    println!("🚀 STARTING AUTO-BENCHMARK SUITE");
    println!("Cycle duration: {}s", args.benchmark_cycle_duration);

    let generators: Vec<(&str, Arc<dyn WorldGenerator>)> = vec![
        ("flat", Arc::new(FlatGenerator) as Arc<dyn WorldGenerator>),
        ("vanilla", Arc::new(VanillaWorldGenerator::new(args.seed)) as Arc<dyn WorldGenerator>),
    ];

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@db:5432/hoppermc".to_string());

    let storage_configs: Vec<(&str, Option<StorageMode>)> = vec![
        ("nostorage", None),
        ("pg_raw", Some(StorageMode::PgRaw)),
        ("pg_jsonb", Some(StorageMode::PgJsonb)),
    ];

    let mut full_report = String::new();
    full_report.push_str("# HopperMC Auto-Benchmark Suite\n\n");

    for (gen_name, gen_arc) in generators {
        for (storage_name, storage_mode) in &storage_configs {
            println!("\n>>> Testing: Gen={} | Storage={}", gen_name, storage_name);
            
            let storage: Option<Arc<dyn ChunkStorage>> = if let Some(mode) = storage_mode {
                match PostgresStorage::new(&database_url, *mode).await {
                    Ok(s) => Some(Arc::new(s) as Arc<dyn ChunkStorage>),
                    Err(e) => {
                        eprintln!("Skipping {} due to connection error: {}", storage_name, e);
                        continue;
                    }
                }
            } else {
                None
            };

            let config_summary = format!("Gen: {} | Storage: {}", gen_name, storage_name);
            let bench = Arc::new(BenchmarkMetrics::new(config_summary));
            let handle = tokio::runtime::Handle::current();
            let vf = Arc::new(VirtualFile::new(gen_arc.clone(), storage.clone(), handle, Some(bench.clone()), args.cache_size, args.prefetch_radius));

            // Stress test: Read spiral of chunks in background
            let vf_clone = vf.clone();
            let stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop_flag_clone = stop_flag.clone();
            
            let _stress_thread = std::thread::spawn(move || {
                let mut radius: i32 = 0;
                while radius < 15 && !stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    for x in -radius..=radius {
                        for z in -radius..=radius {
                            if stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) { break; }
                            // Region header is 8KB, read it first to simulate Minecraft
                            vf_clone.read_at(0, 4096, 0, 0); 
                            
                            // In r.0.0.mca, chunk (x,z) is at get_chunk_file_offset(x, z)
                            let offset = get_chunk_file_offset(x.rem_euclid(32), z.rem_euclid(32));
                            vf_clone.read_at(offset, 4096, 0, 0);
                        }
                        if stop_flag_clone.load(std::sync::atomic::Ordering::Relaxed) { break; }
                    }
                    radius += 1;
                    std::thread::sleep(Duration::from_millis(10));
                }
            });

            tokio::time::sleep(Duration::from_secs(args.benchmark_cycle_duration)).await;
            stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);

            // Record DB size if applicable
            if let Some(s) = &storage {
                if let Ok(size) = s.get_total_size().await {
                    bench.record_db_size(size);
                }
            }

            let report = bench.generate_report();
            full_report.push_str(&format!("## Configuration: {} x {}\n\n```\n{}\n```\n\n", gen_name, storage_name, report));
        }
    }

    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let filename = format!("benchmarks/auto-benchmark-{}.md", timestamp);
    if let Err(e) = std::fs::write(&filename, &full_report) {
        eprintln!("Failed to write auto-benchmark report: {}", e);
    } else {
        println!("🚀 AUTO-BENCHMARK COMPLETE!");
        println!("Combined report written to {}", filename);
    }
}