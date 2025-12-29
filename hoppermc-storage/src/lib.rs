use anyhow::Result;

pub trait ChunkStorage {
    fn read_chunk(&self, chunk_x: i32, chunk_z: i32) -> Result<Option<Vec<u8>>>;
    fn write_chunk(&self, chunk_x: i32, chunk_z: i32, data: &[u8]) -> Result<()>;
}