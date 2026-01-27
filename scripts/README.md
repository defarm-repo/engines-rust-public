# Scripts

Utility scripts for development, testing, deployment, and maintenance.

## Directory Structure

- `development/` - Development and setup scripts
- `deployment/` - Deployment automation scripts
- `testing/` - Testing and validation scripts
- `monitoring/` - Monitoring and health check scripts
- `maintenance/` - Maintenance and repair scripts

## Usage

All scripts should be run from the repository root:

```bash
./scripts/testing/test-api-keys.sh
./scripts/deployment/setup-event-listener-env.sh
```

## Creating New Scripts

1. Place in appropriate category directory
2. Make executable: `chmod +x script.sh`
3. Add shebang: `#!/bin/bash`
4. Add description header
5. Update this README
