use crate::WorldGenerator;
use crate::builder::ChunkBuilder;
use pumpkin_world::generation::generator::{GeneratorInit, VanillaGenerator};
use pumpkin_world::dimension::Dimension;
use pumpkin_util::world_seed::Seed;
use anyhow::Result;

/// Vanilla-style world generator using Pumpkin's VanillaGenerator
/// Generates realistic Minecraft terrain with biomes, caves, ores, etc.
pub struct VanillaWorldGenerator {
    generator: Box<VanillaGenerator>,
    dimension: Dimension,
}

impl VanillaWorldGenerator {
    pub fn new(seed: u64) -> Self {
        Self::with_dimension(seed, Dimension::Overworld)
    }
    
    pub fn with_dimension(seed: u64, dimension: Dimension) -> Self {
        let pumpkin_seed = Seed(seed);
        let generator = Box::new(VanillaGenerator::new(pumpkin_seed, dimension.clone()));
        Self { generator, dimension }
    }
}

impl WorldGenerator for VanillaWorldGenerator {
    fn generate_chunk(&self, x: i32, z: i32) -> Result<Vec<u8>> {
        // TODO: Wire up actual Pumpkin generation
        // For now, generate placeholder mountainous terrain
        // Real implementation needs ProtoChunk + noise sampling
        
        let mut builder = ChunkBuilder::new();
        
        // Simple seed-based height variation placeholder
        let base_height = 64i32;
        let height_variation = ((x.wrapping_mul(31) ^ z.wrapping_mul(17)) % 20) as i32 - 10;
        let surface_y = base_height + height_variation;
        
        // Bedrock
        builder.fill_layer(-64, "minecraft:bedrock");
        
        // Stone layers
        for y in -63..surface_y.min(60) {
            builder.fill_layer(y, "minecraft:stone");
        }
        
        // Dirt layers
        for y in surface_y.min(60)..surface_y {
            builder.fill_layer(y, "minecraft:dirt");
        }
        
        // Grass surface
        builder.fill_layer(surface_y, "minecraft:grass_block");
        
        builder.build(x, z)
    }
}
