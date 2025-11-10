# Public Repository Manifest

This document defines what gets synced to the public repository at https://github.com/defarm-repo/engines-rust-public

## Included Files (Whitelist)

Only the following files and directories are included in the public repository:

### Core Source Code
- `src/` - All Rust source code
- `Cargo.toml` - Package manifest
- `Cargo.lock` - Dependency lock file
- `rust-toolchain.toml` - Rust version specification

### Basic Configuration
- `Dockerfile` - Container build instructions
- `docker-compose.yml` - Container orchestration
- `.gitignore` - Git ignore rules
- `README.md` - Minimal public-facing documentation (auto-generated)

## Excluded Files (Everything Else)

The following are explicitly EXCLUDED from the public repository:

### Documentation
- All `.md` files (except the minimal public README)
- `docs/` directory
- `CLAUDE.md`, `ARCHITECTURE_*.md`, etc.
- API documentation
- Deployment guides
- Internal notes and plans

### Testing & Scripts
- All `.sh` files (shell scripts)
- `tests/` directory
- `scripts/` directory
- Test fixtures and data
- Benchmark files

### Configuration & Secrets
- `.env*` files
- `railway.json`, `railway.toml`
- `nixpacks.toml`
- Platform-specific configs (fly.toml, render.yaml, etc.)
- GitHub workflows (`.github/`)

### Internal Files
- Password reset implementations
- Email configurations
- Demo credentials
- Client-specific documentation
- Migration guides
- Troubleshooting documents
- Session reports
- TODO lists

## Sync Strategy

The workflow uses a **whitelist approach**:
1. Start with empty directory
2. Copy ONLY the allowed files
3. Create minimal public README
4. Remove any accidentally included internal files
5. Force push to public repository

## Security Considerations

- No credentials or API keys
- No internal documentation
- No client names or specific implementations
- No test data or fixtures
- No deployment configurations
- No internal workflow files

## Maintenance

When adding new files to the private repository, consider:
- Should this be public? (Default: NO)
- Does it contain any sensitive information?
- Is it required for external developers to use the code?

Only explicitly add files to the whitelist if they meet ALL criteria:
1. Contains no sensitive information
2. Required for compilation/building
3. Provides value to external developers
4. Does not reveal internal processes