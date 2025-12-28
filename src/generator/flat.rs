

use crate::generator::WorldGenerator;
use crate::generator::builder::ChunkBuilder; // Import our new tool

pub struct FlatGenerator;

impl WorldGenerator for FlatGenerator {
    fn generate_chunk(&self, x: i32, z: i32) -> anyhow::Result<Vec<u8>> {
        let mut builder = ChunkBuilder::new();

        // 1. Bedrock Floor (Y=-64)
        // In our builder, we map Y=-64 to internal logic. 
        // Wait, our builder handles -64..320.
        // Let's just set the bottom layer.
        builder.fill_layer(-64, "minecraft:bedrock");

        // 2. Dirt Layers (-4..-1)
        for y in -4..-1 {
            builder.fill_layer(y, "minecraft:dirt");
        }

        // 3. Grass Block (Y=0)
        builder.fill_layer(0, "minecraft:grass_block");

        // 4. TEST: A Stone Pillar at (8, Y, 8) to prove it's 3D
        for y in 0..10 {
            builder.set_block(8, y, 8, "minecraft:stone");
        }

        let bytes = builder.build(x, z)?;
        Ok(bytes)
    }
}