# DeFarm CLI

Official command-line tool for DeFarm Engines API.

## Installation

```bash
npm install -g defarm-cli
```

Or use via npx (no installation):
```bash
npx defarm-cli login
```

## Quick Start

```bash
# Login
defarm login

# Create a local item
defarm items create --key sisbov --value BR123 --data '{"weight":"500kg"}'

# List circuits
defarm circuits list

# Create a circuit
defarm circuits create "My Supply Chain" --public

# Push item to circuit (tokenization)
defarm circuits push <circuit-id> <local-id>

# View item timeline
defarm items timeline <dfid>
```

## Commands

### Authentication

```bash
defarm login                    # Login to DeFarm
defarm whoami                   # Show current user
defarm logout                   # Logout
```

### Items

```bash
defarm items list               # List all items
defarm items create             # Create local item
defarm items get <dfid>         # Get item details
defarm items timeline <dfid>    # Show item timeline
defarm items storage <dfid>     # Show storage history
```

### Circuits

```bash
defarm circuits list            # List all circuits
defarm circuits create <name>   # Create circuit
defarm circuits get <id>        # Get circuit details
defarm circuits push            # Push item to circuit
defarm circuits items <id>      # List circuit items
defarm circuits members <id>    # List circuit members
```

### Events

```bash
defarm events list <dfid>       # List item events
defarm events create <dfid>     # Create new event
```

### Merkle Tree

```bash
defarm merkle item-root <dfid>          # Get item Merkle root
defarm merkle circuit-root <id>         # Get circuit Merkle root
defarm merkle verify <proof-file>       # Verify Merkle proof
```

### Configuration

```bash
defarm config set <key> <value>  # Set config value
defarm config get <key>          # Get config value
defarm config list               # List all config
```

## Environment Variables

```bash
DEFARM_API_URL      # API base URL (default: https://connect.defarm.net)
DEFARM_TOKEN        # JWT token for authentication
DEFARM_API_KEY      # API key for authentication
```

## Examples

### Complete Workflow

```bash
# 1. Login
defarm login -u your_username -p your_password

# 2. Create local item
defarm items create \
  --namespace bovino \
  --key sisbov \
  --value BR12345678901234 \
  --data '{"breed":"Angus","weight":"500kg"}'

# Output: Local ID: abc-123-def

# 3. Create circuit
defarm circuits create "Cattle Traceability" \
  --description "Track cattle from farm to market" \
  --public \
  --adapter StellarTestnetIpfs

# Output: Circuit ID: xyz-789-uvw

# 4. Push item to circuit (tokenization)
defarm circuits push xyz-789-uvw abc-123-def

# Output: DFID: DFID-20251203-000001-40BA

# 5. Add event
defarm events create DFID-20251203-000001-40BA \
  --type Enriched \
  --visibility Public \
  --metadata '{"action":"vaccination","vaccine":"FMD"}'

# 6. View complete timeline
defarm items timeline DFID-20251203-000001-40BA

# 7. Get Merkle proof (cryptographic verification)
defarm merkle item-root DFID-20251203-000001-40BA
```

### Using API Keys

```bash
# Set API key
export DEFARM_API_KEY="dfm_your_api_key"

# All commands now use API key
defarm items list
defarm circuits list
```

### JSON Output

```bash
# Get JSON output for scripting
defarm items list --json | jq '.[0].dfid'
defarm circuits list --json | jq '.[] | select(.name | contains("Supply"))'
```

## Global Options

```bash
--api-url <url>    # Override API URL
--api-key <key>    # Use API key
--token <token>    # Use JWT token
--json             # Output as JSON
--verbose          # Verbose output
```

## Development

```bash
# Clone repository
git clone https://github.com/defarm/cli
cd cli

# Install dependencies
npm install

# Build
npm run build

# Run locally
npm run dev -- login

# Test
npm test
```

## Support

- Documentation: https://connect.defarm.net/docs
- Issues: https://github.com/defarm/cli/issues
- Email: support@defarm.net

## License

MIT
