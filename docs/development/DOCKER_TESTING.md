# Docker Build Testing

This document explains how to test Docker builds to catch dependency issues before deployment.

## The Problem We Solved

Railway deployment was failing with:
```
error: failed to run custom build command for `libsodium-sys-stable v1.22.4`
thread 'main' panicked at build.rs:534:13:
libsodium not found via pkg-config or vcpkg
```

**Root Cause**: The Dockerfile didn't install `libsodium` which is required by the encryption functionality.

**Solution**: Added `libsodium-dev` (build time) and `libsodium23` (runtime) to Dockerfile.

## How to Prevent This

### 1. Local Testing (Before Pushing)

Run the test script before committing Dockerfile changes:

```bash
./test-docker-build.sh
```

This script tests:
- ✅ Builder stage builds successfully
- ✅ Full image builds successfully
- ✅ libsodium is present in runtime container
- ✅ Binary exists and is executable
- ✅ Binary can run (basic smoke test)
- ✅ Image size is reasonable

### 2. CI/CD Testing (Automatic)

GitHub Actions workflow (`.github/workflows/docker-build.yml`) runs on every push:

- **Docker Build Test**: Builds both stages and verifies libsodium
- **Cargo Test**: Runs unit tests to ensure code compiles with all dependencies

### 3. Manual Docker Testing

Build and test locally:

```bash
# Build the image
docker build -t defarm-api:local .

# Verify libsodium is installed
docker run --rm defarm-api:local sh -c "ldconfig -p | grep libsodium"

# Expected output:
#   libsodium.so.23 (libc6,x86-64) => /lib/x86_64-linux-gnu/libsodium.so.23

# Test the binary exists
docker run --rm defarm-api:local ls -lh /app/defarm-api

# Try running (will fail without DB config, but shouldn't crash)
docker run --rm \
  -e DATABASE_URL=postgresql://test:test@localhost/test \
  defarm-api:local
```

## Common Dependency Issues

### Missing System Libraries

**Symptoms**: Build fails with "library not found" or "command not found"

**Solution**: Add to builder stage in Dockerfile:
```dockerfile
RUN apt-get update && apt-get install -y \
    lib<name>-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
```

**And** to runtime stage:
```dockerfile
RUN apt-get update && apt-get install -y \
    lib<name>23 \
    && rm -rf /var/lib/apt/lists/*
```

### Build vs Runtime Dependencies

| Stage | Package Type | Example | Purpose |
|-------|-------------|---------|---------|
| Builder | `-dev` | `libsodium-dev` | Headers for compilation |
| Builder | tools | `pkg-config` | Find libraries during build |
| Runtime | library | `libsodium23` | Shared library for execution |

### How to Find Package Names

1. Search Debian packages: https://packages.debian.org/
2. Check error message for library name (e.g., `libsodium`)
3. Dev package: `lib<name>-dev`
4. Runtime package: `lib<name><version>` (check available versions)

## Testing Checklist

Before pushing Dockerfile changes:

- [ ] Run `./test-docker-build.sh` locally
- [ ] Verify all tests pass
- [ ] Check image size is reasonable (< 500MB for runtime)
- [ ] Test the binary can start (even if it exits due to missing config)
- [ ] Commit and push - GitHub Actions will run additional tests

## Debugging Failed Builds

### Build fails at dependency compilation

```bash
# Build just the builder stage to see detailed errors
docker build --target builder -t debug-builder .

# Check what packages are available
docker run --rm debug-builder apt-cache search libsodium

# Try installing the package manually
docker run --rm -it debug-builder bash
# Inside container:
apt-get update
apt-get install -y libsodium-dev
pkg-config --libs libsodium  # Should show library path
```

### Runtime fails with "library not found"

```bash
# Check what libraries are linked
docker run --rm defarm-api:test ldd /app/defarm-api

# Check if library is installed
docker run --rm defarm-api:test ldconfig -p | grep <library>

# If missing, add to runtime stage Dockerfile
```

## Example: Adding a New Dependency

Let's say you add a crate that requires `libpq` (PostgreSQL client library):

1. **Update Dockerfile builder stage**:
```dockerfile
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libsodium-dev \
    libpq-dev \        # <-- Add this
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
```

2. **Update Dockerfile runtime stage**:
```dockerfile
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsodium23 \
    libpq5 \            # <-- Add this
    curl \
    && rm -rf /var/lib/apt/lists/*
```

3. **Test locally**:
```bash
./test-docker-build.sh
```

4. **Verify the library**:
```bash
docker run --rm defarm-api:test ldconfig -p | grep libpq
```

5. **Commit and push** - CI will validate

## Reference

- [Debian Package Search](https://packages.debian.org/)
- [Rust Docker Best Practices](https://docs.docker.com/language/rust/)
- [Multi-stage Build Guide](https://docs.docker.com/build/building/multi-stage/)
