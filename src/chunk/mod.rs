//! Chunk generation and management.
//!
//! This module handles:
//! - Procedural chunk generation
//! - Chunk data serialization (NBT + compression)
//! - Chunk provider that combines storage and generation

mod generator;

pub use generator::Generator;

use std::sync::Arc;
use crate::storage::{ChunkPos, ChunkStorage};

/// Provides chunks by checking storage first, then falling back to generation.
///
/// This is the main interface for getting chunk data:
/// 1. Check if chunk exists in storage (was modified by player)
/// 2. If not, generate it procedurally
pub struct ChunkProvider<S: ChunkStorage> {
    storage: Arc<S>,
    generator: Generator,
}

impl<S: ChunkStorage> ChunkProvider<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            generator: Generator::new(),
        }
    }

    /// Get chunk data (from storage or generate new).
    /// Returns raw MCA-formatted bytes (length + compression type + compressed NBT).
    pub fn get_chunk(&self, pos: ChunkPos) -> Vec<u8> {
        // First check storage for modified chunks
        if let Some(data) = self.storage.get(pos) {
            return data;
        }

        // Generate new chunk
        self.generator.generate(pos.x, pos.z)
    }

    /// Save chunk data to storage.
    pub fn save_chunk(&self, pos: ChunkPos, data: Vec<u8>) {
        self.storage.set(pos, data);
    }

    /// Check if chunk has been modified (exists in storage).
    pub fn is_modified(&self, pos: ChunkPos) -> bool {
        self.storage.exists(pos)
    }
}
