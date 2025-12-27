# CLAUDE.md

Guidance for Claude Code when working with this repository.

## Project Overview

**mc-anvil-db** is a Rust FUSE filesystem that generates Minecraft world chunks procedurally. The server reads virtual `.mca` region files, and we generate chunk data on-the-fly.

## Build Commands

```bash
# Local development
cargo build --release
cargo check              # Fast type checking
cargo test               # Run tests

# Docker
docker compose up --build
docker compose logs -f mc-anvil-db
```

## Architecture

### Module Structure

```
src/
├── main.rs           # Entry point, mounts FUSE
├── fuse/             # FUSE filesystem layer
│   ├── mod.rs        # AnvilFS struct, Filesystem trait impl
│   └── inode.rs      # Inode ↔ RegionPos mapping
├── region/           # MCA file format
│   ├── mod.rs        # RegionPos, coordinate helpers
│   └── header.rs     # Location/timestamp table generation
├── chunk/            # Chunk operations
│   ├── mod.rs        # ChunkProvider (storage + generator)
│   └── generator.rs  # Procedural generation (flat world)
├── storage/          # Storage backends
│   ├── mod.rs        # ChunkStorage trait
│   └── memory.rs     # HashMap-based (dev/testing)
└── nbt.rs            # NBT structs for fastnbt serialization
```

### Key Abstractions

1. **ChunkStorage trait** (`storage/mod.rs`)
   - Abstract interface for any storage backend
   - Implementations: MemoryStorage, (TODO) RedisStorage, PostgresStorage

2. **ChunkProvider** (`chunk/mod.rs`)
   - Combines storage lookup with procedural generation
   - First checks storage, then falls back to generator

3. **AnvilFS** (`fuse/mod.rs`)
   - Implements fuser::Filesystem trait
   - Handles FUSE callbacks (read, write, lookup, etc.)

4. **RegionPos** (`region/mod.rs`)
   - Represents region file coordinates (parsed from `r.X.Z.mca`)

### Data Flow

```
Minecraft read request
    ↓
FUSE lookup ("r.0.0.mca") → InodeMap → inode
    ↓
FUSE read (inode, offset, size)
    ↓
AnvilFS.read_region()
    ├── Header zone (0-8191): Header::generate()
    └── Chunk zone (8192+): ChunkProvider.get_chunk()
                                ├── Check MemoryStorage
                                └── Generate if not found
    ↓
Return bytes to Minecraft
```

## Key Dependencies

- `fuser` - FUSE bindings
- `fastnbt` - NBT serialization
- `flate2` - Zlib compression
- `serde` - Serialization framework

## Docker Setup

Three containers:
1. **mc-anvil-db** - FUSE driver (privileged, /dev/fuse)
2. **redis** - Storage backend (TODO: integrate)
3. **minecraft** - Paper server

FUSE requires:
- `cap_add: SYS_ADMIN`
- `devices: /dev/fuse`
- `propagation: rshared` for bind mounts
- `apparmor:unconfined`

## Development Notes

### Adding a New Storage Backend

1. Create `src/storage/redis.rs`
2. Implement `ChunkStorage` trait
3. Add to `src/storage/mod.rs`
4. Use in `main.rs`

### Chunk Format

MCA file structure:
- Bytes 0-4095: Location table (1024 × 4 bytes)
- Bytes 4096-8191: Timestamp table
- Bytes 8192+: Chunk data (sectors)

Each chunk entry: `[length:4][compression:1][nbt:N]`

### Testing

```bash
# Unit tests
cargo test

# Manual FUSE test
mkdir /tmp/test-mount
cargo run &
ls /tmp/test-mount/
cat /tmp/test-mount/r.0.0.mca | xxd | head
fusermount -u /tmp/test-mount
```

## Common Issues

1. **FUSE mount stuck after crash**
   ```bash
   sudo fusermount -uz /path/to/mount
   ```

2. **Permission denied on /dev/fuse**
   - Add user to `fuse` group
   - Or run with sudo

3. **Paper "truncated header" errors**
   - Usually means read() didn't return enough bytes
   - Check zone handling in `AnvilFS.read_region()`
