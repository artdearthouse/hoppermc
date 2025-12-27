
use std::collections::HashMap;
use crate::chunk::{ChunkRoot, Section, BlockStates, BlockState, Biomes}; // Correct path?
// Check imports in src/chunk.rs once creating. Assuming crate::chunk for now as per previous edits.

#[derive(Default)]
pub struct ChunkBuilder {
    // We store blocks in a sparse map for simplicity in MVP.
    // Key: (x, y, z), Value: Block Name
    // This isn't the most efficient (VoxelGrid is faster), but it's the easiest to write "set_block".
    // For full layers we will handle efficient filling during build().
    custom_blocks: HashMap<(u8, i32, u8), String>,
    
    // Optimisation for layers:
    // Key: y, Value: Block Name
    full_layers: HashMap<i32, String>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a single block at chunk-local coordinates (x: 0..15, z: 0..15)
    pub fn set_block(&mut self, x: u8, y: i32, z: u8, name: &str) {
        if x < 16 && z < 16 {
            self.custom_blocks.insert((x, y, z), name.to_string());
        }
    }

    /// Fill an entire Y-layer with a block (efficiently)
    pub fn fill_layer(&mut self, y: i32, name: &str) {
        self.full_layers.insert(y, name.to_string());
        // Remove individual blocks at this Y to save memory/logic, they are overwritten
        self.custom_blocks.retain(|(_, by, _), _| *by != y);
    }

    /// Build the ChunkRoot NBT structure
    pub fn build(self, chunk_x: i32, chunk_z: i32) -> ChunkRoot {
        let mut sections = Vec::new();

        // Minecraft world height: usually -64 to 320 -> Sections -4 to 19
        for sec_y in -4..20 { // 24 sections
            sections.push(self.build_section(sec_y));
        }

        // Get data version from env or default
         let data_version = std::env::var("MC_DATA_VERSION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4671);

        ChunkRoot {
            data_version,
            x_pos: chunk_x,
            y_pos: -4, // Bottom of the world
            z_pos: chunk_z,
            status: "minecraft:features".to_string(),
            last_update: 0,
            sections,
        }
    }

    fn build_section(&self, sec_y: i8) -> Section {
        // Calculate Y range for this section
        let start_y = (sec_y as i32) * 16;
        let end_y = start_y + 16;

        // 1. Check if section is completely empty (optimization)
        // If no layers set in this range AND no custom blocks in this range -> Empty
        let has_layers = (start_y..end_y).any(|y| self.full_layers.contains_key(&y));
        let has_blocks = self.custom_blocks.keys().any(|(_, y, _)| *y >= start_y && *y < end_y);

        if !has_layers && !has_blocks {
            // Return empty section (Air)
            return Section {
                y: sec_y,
                block_states: None, // Implicit Air
                biomes: Some(Biomes {
                    palette: vec!["minecraft:plains".to_string()],
                    data: None, // Uniform biome
                }),
            };
        }

        // 2. Build Palette and Data
        // Collect all 4096 blocks for this section
        let mut palette = Vec::new();
        let mut name_to_index = HashMap::new();
        let mut block_indices = Vec::with_capacity(4096);

        // Standard Order: Y lines of X columns (Y -> Z -> X) => Index = (y*16 + z)*16 + x
        for y in 0..16 {
            let world_y = start_y + y;
            for z in 0..16 {
                for x in 0..16 {
                    // Determine block at this pos
                    let block_name = if let Some(name) = self.custom_blocks.get(&(x as u8, world_y, z as u8)) {
                        name.clone()
                    } else if let Some(layer_name) = self.full_layers.get(&world_y) {
                        layer_name.clone()
                    } else {
                        "minecraft:air".to_string()
                    };

                    // Add to palette if new
                    let idx = *name_to_index.entry(block_name.clone()).or_insert_with(|| {
                        let i = palette.len();
                        palette.push(block_name);
                        i
                    });
                    block_indices.push(idx);
                }
            }
        }

        // 3. Optimize: If uniform, return valid single-palette section
        if palette.len() <= 1 {
            let single_block = palette.first().cloned().unwrap_or("minecraft:air".to_string());
            return Section {
                y: sec_y,
                block_states: Some(BlockStates {
                    palette: vec![BlockState { name: single_block }],
                    data: None,
                }),
                biomes: Some(Biomes {
                    palette: vec!["minecraft:plains".to_string()],
                    data: None,
                }),
            };
        }

        // 4. Bit Packing (Mandatory for Palette > 1)
        // Minecraft 1.16+: Compact Long Array.
        // Bits per block = ceil(log2(palette_len)), min 4.
        let mut bits_per_block = (palette.len() as f64).log2().ceil() as usize;
        if bits_per_block < 4 { bits_per_block = 4; }
        
        // Data len = ceil(4096 * bits / 64)
        let data_len = (4096 * bits_per_block + 63) / 64;
        let mut loaded_data = vec![0i64; data_len];

        // Packing loop
        let blocks_per_long = 64 / bits_per_block;
        let mask = (1u64 << bits_per_block) - 1;

        for (i, &block_idx) in block_indices.iter().enumerate() {
            let long_index = i / blocks_per_long;
            let sub_index = i % blocks_per_long;
            let bit_offset = sub_index * bits_per_block;

            // We need to treat i64 as u64 for bitwise ops, then cast back
            let val = (block_idx as u64) & mask;
            let current = loaded_data[long_index] as u64;
            let updated = current | (val << bit_offset);
            loaded_data[long_index] = updated as i64;
        }

        Section {
            y: sec_y,
            block_states: Some(BlockStates {
                palette: palette.into_iter().map(|n| BlockState { name: n }).collect(),
                data: Some(fastnbt::LongArray::new(loaded_data)),
            }),
            biomes: Some(Biomes {
                palette: vec!["minecraft:plains".to_string()],
                data: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_empty_section() {
        let builder = ChunkBuilder::new();
        let section = builder.build_section(0); // Y=0
        
        // Empty section -> block_states is None (Implicit Air)
        assert!(section.block_states.is_none());
        assert!(section.biomes.is_some()); // Biomes still present
    }

    #[test]
    fn test_builder_single_block_uniform() {
        let mut builder = ChunkBuilder::new();
        // Fill all 16 layers of section 0 (Y=0..16) with Stone to make it uniform
        for y in 0..16 {
            builder.fill_layer(y, "minecraft:stone");
        }
        
        let section = builder.build_section(0);
        let states = section.block_states.unwrap();
        
        // Uniform section -> Palette size 1, No data
        assert_eq!(states.palette.len(), 1);
        assert_eq!(states.palette[0].name, "minecraft:stone");
        assert!(states.data.is_none());
    }

    #[test]
    fn test_builder_mixed_blocks_bit_packing() {
        let mut builder = ChunkBuilder::new();
        
        // Place just two blocks in Y=0 section to force palette creation
        // (0,0,0) -> Stone
        // (1,0,0) -> Dirt
        // Rest -> Air (default)
        builder.set_block(0, 0, 0, "minecraft:stone");
        builder.set_block(1, 0, 0, "minecraft:dirt");
        
        let section = builder.build_section(0);
        let states = section.block_states.unwrap();
        
        // Palette should contain: Air, Stone, Dirt
        // Note: Hashmap iteration order is random, but length is fixed
        assert_eq!(states.palette.len(), 3);
        
        // Data must be present
        assert!(states.data.is_some());
        let long_array = states.data.unwrap();
        
        // 3 items -> ceil(log2(3)) = 2 bits. Min is 4 bits.
        // So we expect 4 bits per block.
        // Total blocks = 4096. 
        // Longs needed = ceil(4096 * 4 / 64) = 256 longs.
        assert_eq!(long_array.len(), 256);
    }
    
    #[test]
    fn test_bits_calculation_min_4() {
        let mut builder = ChunkBuilder::new();
        // Add 2 different blocks (Air is implicit 3rd)
        builder.set_block(0, 0, 0, "minecraft:a");
        builder.set_block(0, 0, 1, "minecraft:b");
        
        let section = builder.build_section(0);
        let states = section.block_states.as_ref().unwrap();
        let _pal_len = states.palette.len(); // Suppress warning
        
        // Palette: Air, A, B (3 items). 
        // log2(3) = 1.58 -> 2 bits.
        // Min is 4.
        
        // We can verify this by checking data length.
        // 4096 blocks * 4 bits = 16384 bits.
        // 16384 / 64 = 256 longs.
        let data_len = states.data.as_ref().unwrap().len();
        assert_eq!(data_len, 256);
    }
}
