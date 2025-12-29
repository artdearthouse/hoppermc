# HopperMC

A FUSE-based virtual filesystem for Minecraft that intercepts and simulates `.mca` region files. It provides a programmable storage layer for the Anvil format, enabling on-the-fly chunk generation and virtualized world management with zero local disk footprint.

![Infinite Flat World Demo](demo/infinity_flat_demo.png)

## Overview

Currently, this project acts as a **Stateless Infinite Flat World Generator**.

**Key Features:**
- [x] 🚀 **Infinite World**: Generates chunks procedurally as Minecraft requests them (Stateless).
- [x] 🔄 **Negative Coordinates**: Fully supports infinite exploration in all directions (negative X/Z).
- [x] 🎃 **Pumpkin-Powered Generator**: Uses [Pumpkin-MC](https://github.com/Pumpkin-MC/Pumpkin) for robust and efficient chunk generation and NBT serialization.
- [x] 📁 **Anvil Format**: Emulates standard Minecraft region headers and chunk data (Works with Paper 1.21+).
- [x] 🐳 **Docker-first**: Runs in a container with FUSE permissions (`/dev/fuse`).
- [x] ⚡ **Fast Builds**: Docker pipeline optimized with Workspace Cache Mounts.
- [x] 🛠 **Generic File Support**: Handles auxiliary files (like backups) gracefully to prevent server crashes.

## Vision & Goals

This project is not just a filesystem; it is a **Universal Storage Middleware** for Minecraft. By intercepting I/O at the OS level, we decouple the game engine from physical storage.

**Our long-term goals:**
1.  **Storage Agnostic**: Store your world anywhere—Postgres, Redis, S3, or even distributed across a network.
2.  **Stateless Gaming**: Treating Minecraft servers as ephemeral compute nodes while keeping world data persistent and shared.
3.  **P2P & Distributed Worlds**: Enabling multiple servers to simulate different parts of the same continuous world (sharding), paving the way for true MMO-scale Minecraft architecture.
4.  **Universal Compatibility**: Works with any server core (Vanilla, Paper, Fabric, Forge) because it operates below the application layer.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Minecraft Server                   │
│                    (Paper 1.21+)                    │
└─────────────────────┬───────────────────────────────┘
                      │ reads "r.x.z.mca"
                      ▼
┌─────────────────────────────────────────────────────┐
│                    FUSE Layer                       │
│                (hoppermc-fs crate)                  │
│            Intercepts File I/O                      │
├─────────────────────────────────────────────────────┤
│                 World Generator                     │
│                (hoppermc-gen crate)                 │
│          Generates chunks on-the-fly                │
├─────────────────────────────────────────────────────┤
│                 Pumpkin Backend                     │
│               (Pumpkin World Lib)                   │
│         Handles Chunk Structure & NBT               │
└─────────────────────────────────────────────────────┘
```

## Project Structure (Cargo Workspace)

```
.
├── Cargo.toml        # Workspace Root
├── hoppermc/         # CLI Entry Point & FUSE Mount
├── hoppermc-anvil/   # Anvil Format Details (Offsets, Headers, Compression)
├── hoppermc-fs/      # FUSE Filesystem Implementation (Inodes, Virtual Files)
├── hoppermc-gen/     # World Generation & Pumpkin Integration
└── hoppermc-storage/ # Data Persistence Layers (Postgres, etc.)
```

## Storage Backends

| Backend | Status | Use Case |
|---------|--------|----------|
| **Stateless Generator** | ✅ **Active** | Infinite flat world, testing |
| `PostgresStorage` | 🛠 **Ready for Dev** | Environment included, code structures present |
| `MemoryStorage` | 🚧 Planned | Fast temporary storage |

## Quick Start

### With Docker (Recommended)

```bash
# 1. Start the FUSE filesystem and Minecraft server (Fast Build)
DOCKER_BUILDKIT=1 docker compose up -d --build

# 2. Connect to localhost:25565
```

This starts:
- `hoppermc`: The FUSE filesystem mounting to `/mnt/region`.
- `minecraft`: A Paper server configured to use the FUSE mount.

**Note:** Any blocks you place or destroy **will NOT be saved** in the current "Stateless" mode. The server "writes" the data, but the FUSE layer simply acknowledges the write without persisting it.

## How It Works

1. **Minecraft requests `r.x.z.mca`**: FUSE intercepts the `open` and `read` calls.
2. **Header Generation**: FUSE calculates where chunks *would* be in a real file and sends a generated header.
3. **Chunk Mapping**: It calculates which chunk (X, Z) corresponds to the requested file offset.
4. **Pumpkin Integration**: `FlatGenerator` asks `ChunkBuilder` to build the chunk.
5. **Serialization**: `builder.rs` uses **Pumpkin-World** to create and serialize the complex NBT structure (including palettes, sections, and lighting).
6. **Compression**: The chunk is Zlib-compressed and sent to Minecraft.

## Troubleshooting

-   **"Transparent Chunks"**: If you see transparent chunks that you can walk on, it usually means the server read "0 bytes" (EOF) unexpectedly. This has been fixed in v0.0.3 by correcting inode packing logic.
-   **Panic on Join**: If `hoppermc` crashes with `index out of bounds` in `pumpkin-world`, ensure you are initializing `ChunkLight` with 24 sections in the builder (Fixed in recent updates).

## Acknowledgments

Special thanks to the **[Pumpkin-MC Team](https://github.com/Pumpkin-MC/Pumpkin)**!
We utilize their excellent crates (`pumpkin-world`, `pumpkin-data`, `pumpkin-nbt`) to handle standard-compliant Minecraft NBT Serialization and Data Structures.

## License

MIT
