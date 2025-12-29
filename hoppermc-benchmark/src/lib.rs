use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct BenchmarkMetrics {
    // Generation Stats
    pub total_chunks_generated: AtomicUsize,
    pub total_generation_time_us: AtomicU64,
    pub max_generation_time_us: AtomicU64,
    
    // Storage Stats
    pub total_chunks_loaded: AtomicUsize,
    pub total_load_time_us: AtomicU64,
    pub total_chunks_saved: AtomicUsize,
    pub total_save_time_us: AtomicU64,

    // Detailed Breakdown
    pub total_generation_biomes_us: AtomicU64,
    pub total_generation_noise_us: AtomicU64, // Terrain noise
    pub total_generation_surface_us: AtomicU64,
    pub total_generation_conversion_us: AtomicU64,
    
    pub total_serialization_us: AtomicU64,
    pub total_compression_us: AtomicU64,
    
    // FUSE Stats
    // FUSE Stats
    pub total_fuse_read_count: AtomicUsize,
    pub total_fuse_read_time_us: AtomicU64,
    pub total_fuse_bytes_sent: AtomicUsize,
    
    pub total_gen_bytes_raw: AtomicUsize,
    pub total_gen_bytes_compressed: AtomicUsize,
    
    // Cache
    pub total_cache_hits: AtomicUsize,
    pub total_cache_misses: AtomicUsize,

    // Session
    pub start_time: Option<Instant>,
    pub config_summary: String,
}

impl BenchmarkMetrics {
    pub fn new(config_summary: String) -> Self {
        Self {
            start_time: Some(Instant::now()),
            config_summary,
            ..Default::default()
        }
    }

    pub fn record_generation(&self, duration: Duration) {
        self.total_chunks_generated.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.total_generation_time_us.fetch_add(us, Ordering::Relaxed);
        self.max_generation_time_us.fetch_max(us, Ordering::Relaxed);
    }

    pub fn record_load(&self, duration: Duration) {
        self.total_chunks_loaded.fetch_add(1, Ordering::Relaxed);
        self.total_load_time_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_save(&self, duration: Duration) {
        self.total_chunks_saved.fetch_add(1, Ordering::Relaxed);
        self.total_save_time_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_generation_biomes(&self, duration: Duration) {
        self.total_generation_biomes_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_generation_noise(&self, duration: Duration) {
        self.total_generation_noise_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
    
    pub fn record_generation_surface(&self, duration: Duration) {
        self.total_generation_surface_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }
    
    pub fn record_generation_conversion(&self, duration: Duration) {
        self.total_generation_conversion_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_serialization(&self, duration: Duration) {
        self.total_serialization_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_compression(&self, duration: Duration) {
        self.total_compression_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_fuse_request(&self, duration: Duration, bytes_sent: usize) {
        self.total_fuse_read_count.fetch_add(1, Ordering::Relaxed);
        self.total_fuse_read_time_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.total_fuse_bytes_sent.fetch_add(bytes_sent, Ordering::Relaxed);
    }

    pub fn record_chunk_sizes(&self, raw: usize, compressed: usize) {
        self.total_gen_bytes_raw.fetch_add(raw, Ordering::Relaxed);
        self.total_gen_bytes_compressed.fetch_add(compressed, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.total_cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.total_cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn generate_report(&self) -> String {
        let uptime = self.start_time.unwrap_or_else(Instant::now).elapsed();
        let generated = self.total_chunks_generated.load(Ordering::Relaxed);
        let gen_time_total = self.total_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_max = self.max_generation_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let gen_avg = if generated > 0 { gen_time_total / generated as f64 } else { 0.0 };
        
        // Granular stats
        let biome_time = self.total_generation_biomes_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let noise_time = self.total_generation_noise_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let surface_time = self.total_generation_surface_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let conv_time = self.total_generation_conversion_us.load(Ordering::Relaxed) as f64 / 1000.0;
        
        let ser_time = self.total_serialization_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let comp_time = self.total_compression_us.load(Ordering::Relaxed) as f64 / 1000.0;
        
        let biome_avg = if generated > 0 { biome_time / generated as f64 } else { 0.0 };
        let noise_avg = if generated > 0 { noise_time / generated as f64 } else { 0.0 };
        let surface_avg = if generated > 0 { surface_time / generated as f64 } else { 0.0 };
        let conv_avg = if generated > 0 { conv_time / generated as f64 } else { 0.0 };
        
        let ser_avg = if generated > 0 { ser_time / generated as f64 } else { 0.0 };
        let comp_avg = if generated > 0 { comp_time / generated as f64 } else { 0.0 };

        // Cache stats
        let hits = self.total_cache_hits.load(Ordering::Relaxed);
        let misses = self.total_cache_misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate = if total_requests > 0 { (hits as f64 / total_requests as f64) * 100.0 } else { 0.0 };

        let loaded = self.total_chunks_loaded.load(Ordering::Relaxed);
        let load_time = self.total_load_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let load_avg = if loaded > 0 { load_time / loaded as f64 } else { 0.0 };
        
        let saved = self.total_chunks_saved.load(Ordering::Relaxed);
        let save_time = self.total_save_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let save_avg = if saved > 0 { save_time / saved as f64 } else { 0.0 };

        // FUSE stats
        let fuse_requests = self.total_fuse_read_count.load(Ordering::Relaxed);
        let fuse_time = self.total_fuse_read_time_us.load(Ordering::Relaxed) as f64 / 1000.0; // ms
        let fuse_avg_latency = if fuse_requests > 0 { fuse_time / fuse_requests as f64 } else { 0.0 };
        
        // Overhead relative to generation (rough estimate, assumes 1 gen per req in miss case, or 0 in hit)
        // A better metric is Latency - Gen Avg (if generated).
        let fuse_overhead = fuse_avg_latency - gen_avg; 
        
        let fuse_throughput = if uptime.as_secs_f64() > 0.0 {
            (self.total_fuse_bytes_sent.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0) / uptime.as_secs_f64()
        } else { 0.0 };
        
        let gen_raw = self.total_gen_bytes_raw.load(Ordering::Relaxed);
        let gen_comp = self.total_gen_bytes_compressed.load(Ordering::Relaxed);
        
        // Avg Chunk Sizes
        let avg_raw_kb = if generated > 0 { gen_raw as f64 / generated as f64 / 1024.0 } else { 0.0 };
        let avg_comp_kb = if generated > 0 { gen_comp as f64 / generated as f64 / 1024.0 } else { 0.0 };
        
        let compression_ratio = if gen_comp > 0 {
            gen_raw as f64 / gen_comp as f64
        } else { 0.0 };

        format!(
            "HopperMC Benchmark Report\n\
             =========================\n\
             Configuration: {}\n\
             Session Duration: {:.2?}\n\n\
             [Generation]\n\
             Chunks Generated: {}\n\
             Total Time: {:.2} ms\n\
             Avg Time: {:.2} ms/chunk\n\
             Max Time: {:.2} ms\n\
               - Logic Breakdown:\n\
                 * Biomes: {:.2} ms\n\
                 * Noise (Terrain): {:.2} ms\n\
                 * Surface Rules: {:.2} ms\n\
                 * Data Conversion: {:.2} ms\n\
               - Serialization: {:.2} ms/chunk\n\
               - Compression: {:.2} ms/chunk\n\n\
             [Storage Read]\n\
             Chunks Loaded: {}\n\
             Avg Time: {:.2} ms/chunk\n\n\
             [Storage Write]\n\
             Chunks Saved: {}\n\
             Avg Time: {:.2} ms/chunk\n\n\
             [FUSE Filesystem]\n\
             Requests: {}\n\
             Avg Latency: {:.2} ms\n\
             Overhead: {:.2} ms/req (Latency - Generation)\n\
             Throughput: {:.2} MB/s\n\
             Compression Ratio: {:.2}x ({:.1} KB -> {:.1} KB)\n\n\
             [Cache]\n\
             Hits: {}\n\
             Misses: {}\n\
             Hit Rate: {:.1}%\n",
            self.config_summary,
            uptime,
            generated, gen_time_total, gen_avg, gen_max,
            biome_avg, noise_avg, surface_avg, conv_avg,
            ser_avg, comp_avg,
            loaded, load_avg,
            saved, save_avg,
            // FUSE Params
            fuse_requests, fuse_avg_latency, fuse_overhead, fuse_throughput, 
            compression_ratio, avg_raw_kb, avg_comp_kb,
            hits, misses, hit_rate
        )
    }
}
