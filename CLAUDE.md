# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

mc-anvil-db is a Rust-based FUSE filesystem for Minecraft worlds. It uses fuser to mount Minecraft world data, with fastnbt for NBT parsing and flate2 for compression. Redis is used for data storage/caching.

## Build Commands

```bash
# Local development
cargo build --release

# Docker build and run
docker compose up --build

# Run specific service
docker compose up mc-anvil-db
```

## Architecture

**Three-container Docker setup:**
- `mc-anvil-db` - Rust FUSE filesystem that mounts at `/mnt/world`
- `redis` - Data storage with AOF persistence
- `minecraft` - Paper Minecraft server (itzg/minecraft-server)

**Key dependencies:**
- `fuser` - FUSE bindings for Rust
- `fastnbt` - Minecraft NBT format parsing
- `flate2` - Compression for world data

**Container requirements:**
- FUSE requires `SYS_ADMIN` capability and `/dev/fuse` device
- Uses `rshared` bind propagation for FUSE mounts
- AppArmor unconfined for FUSE operations

## Environment Variables

- `REDIS_URL` - Redis connection string (default: `redis://redis:6379`)
- `RUST_LOG` - Logging level (default: `info`)
