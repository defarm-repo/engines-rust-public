#!/bin/bash
# DeFarm Engines - TypeScript SDK Generator
# This script generates a TypeScript SDK from the OpenAPI specification

set -e

echo "🚀 DeFarm Engines TypeScript SDK Generator"
echo "=========================================="
echo ""

# Configuration
OPENAPI_FILE="../docs/api/openapi.yaml"
OUTPUT_DIR="./typescript"
PACKAGE_NAME="@defarm/sdk"
PACKAGE_VERSION="1.0.0"

# Check if OpenAPI file exists
if [ ! -f "$OPENAPI_FILE" ]; then
    echo "❌ Error: OpenAPI file not found at $OPENAPI_FILE"
    exit 1
fi

echo "✓ OpenAPI specification found"

# Check if npx is available
if ! command -v npx &> /dev/null; then
    echo "❌ Error: npx not found. Please install Node.js and npm"
    exit 1
fi

echo "✓ Node.js environment detected"

# Clean output directory
if [ -d "$OUTPUT_DIR" ]; then
    echo "🧹 Cleaning existing output directory..."
    rm -rf "$OUTPUT_DIR"
fi

mkdir -p "$OUTPUT_DIR"
echo "✓ Output directory prepared: $OUTPUT_DIR"

# Generate TypeScript SDK using openapi-typescript-codegen
echo ""
echo "📦 Generating TypeScript SDK..."
echo ""

npx openapi-typescript-codegen \
    --input "$OPENAPI_FILE" \
    --output "$OUTPUT_DIR" \
    --client axios \
    --name DefarmClient \
    --useOptions \
    --useUnionTypes \
    --exportCore true \
    --exportServices true \
    --exportModels true \
    --exportSchemas false

echo ""
echo "✓ SDK generation complete!"

# Create package.json
echo ""
echo "📝 Creating package.json..."

cat > "$OUTPUT_DIR/package.json" << EOF
{
  "name": "$PACKAGE_NAME",
  "version": "$PACKAGE_VERSION",
  "description": "Official TypeScript SDK for DeFarm Engines API",
  "main": "index.js",
  "types": "index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest",
    "prepublishOnly": "npm run build"
  },
  "keywords": [
    "defarm",
    "traceability",
    "blockchain",
    "supply-chain",
    "api-client"
  ],
  "author": "DeFarm",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/defarm/sdk-typescript"
  },
  "dependencies": {
    "axios": "^1.6.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.3.0",
    "jest": "^29.7.0",
    "@types/jest": "^29.5.0"
  }
}
EOF

echo "✓ package.json created"

# Create README.md for the SDK
echo ""
echo "📖 Creating README.md..."

cat > "$OUTPUT_DIR/README.md" << 'EOF'
# DeFarm Engines TypeScript SDK

Official TypeScript/JavaScript SDK for the DeFarm Engines API.

## Installation

```bash
npm install @defarm/sdk
# or
yarn add @defarm/sdk
```

## Quick Start

```typescript
import { DefarmClient } from '@defarm/sdk';

// Initialize with JWT token
const client = new DefarmClient({
  BASE: 'https://connect.defarm.net',
  TOKEN: 'your-jwt-token'
});

// Or with API key
const client = new DefarmClient({
  BASE: 'https://connect.defarm.net',
  HEADERS: {
    'X-API-Key': 'dfm_your_api_key'
  }
});

// Login
const { token } = await client.auth.login({
  username: 'your_username',
  password: 'your_password'
});

// Update client with token
client.request.config.TOKEN = token;

// List circuits
const circuits = await client.circuits.listCircuits();

// Create local item
const item = await client.items.createLocalItem({
  identifiers: [{
    namespace: 'bovino',
    key: 'sisbov',
    value: 'BR12345678901234',
    id_type: 'Canonical',
    verified: false
  }],
  enriched_data: {
    weight: '500kg',
    breed: 'Angus'
  }
});

// Push to circuit
const result = await client.circuits.pushLocalItemToCircuit({
  circuitId: 'your-circuit-id',
  requestBody: {
    local_id: item.local_id
  }
});

console.log('Item tokenized:', result.dfid);
```

## Features

- ✅ Fully typed with TypeScript
- ✅ Auto-generated from OpenAPI specification
- ✅ Supports both JWT and API Key authentication
- ✅ Promise-based async/await API
- ✅ Comprehensive error handling
- ✅ Built on Axios for HTTP requests

## API Coverage

All DeFarm Engines API endpoints:
- Authentication
- Items (local and tokenized)
- Circuits (creation, operations, webhooks)
- Events (creation, querying)
- Merkle State Tree (proofs, verification)
- Snapshots
- Timeline
- Admin operations
- And more...

## Documentation

Full API documentation: https://connect.defarm.net/docs

## Support

- Issues: https://github.com/defarm/sdk-typescript/issues
- Email: support@defarm.net

## License

MIT
EOF

echo "✓ README.md created"

# Create tsconfig.json
echo ""
echo "⚙️  Creating tsconfig.json..."

cat > "$OUTPUT_DIR/tsconfig.json" << EOF
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "declaration": true,
    "outDir": "./dist",
    "rootDir": "./",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "moduleResolution": "node",
    "resolveJsonModule": true
  },
  "include": [
    "./**/*"
  ],
  "exclude": [
    "node_modules",
    "dist"
  ]
}
EOF

echo "✓ tsconfig.json created"

# Create example usage file
echo ""
echo "📝 Creating example..."

cat > "$OUTPUT_DIR/example.ts" << 'EOF'
import { DefarmClient } from './index';

async function main() {
  // Initialize client
  const client = new DefarmClient({
    BASE: 'https://connect.defarm.net'
  });

  try {
    // Login
    console.log('Logging in...');
    const { token, user_id } = await client.auth.login({
      username: 'hen',
      password: 'demo123'
    });

    // Update client with token
    client.request.config.TOKEN = token;
    console.log('✓ Logged in as:', user_id);

    // Create local item
    console.log('\nCreating local item...');
    const item = await client.items.createLocalItem({
      identifiers: [{
        namespace: 'test',
        key: 'example',
        value: 'SDK-' + Date.now(),
        id_type: 'Contextual',
        verified: false
      }],
      enriched_data: {
        source: 'typescript-sdk-example',
        timestamp: new Date().toISOString()
      }
    });
    console.log('✓ Item created with LID:', item.local_id);

    // List circuits
    console.log('\nListing circuits...');
    const circuits = await client.circuits.listCircuits();
    console.log('✓ Found', circuits.length, 'circuits');

    if (circuits.length > 0) {
      const circuit = circuits[0];
      console.log('  First circuit:', circuit.name);

      // Push item to circuit
      console.log('\nPushing item to circuit...');
      const result = await client.circuits.pushLocalItemToCircuit({
        circuitId: circuit.circuit_id,
        requestBody: {
          local_id: item.local_id
        }
      });
      console.log('✓ Item tokenized with DFID:', result.dfid);

      // Get item timeline
      console.log('\nFetching timeline...');
      const timeline = await client.timeline.getItemTimeline({
        dfid: result.dfid
      });
      console.log('✓ Timeline has', timeline.timeline.length, 'entries');
    }

    console.log('\n✅ Example completed successfully!');
  } catch (error) {
    console.error('❌ Error:', error);
  }
}

main();
EOF

echo "✓ example.ts created"

# Summary
echo ""
echo "=========================================="
echo "✅ TypeScript SDK Generated Successfully!"
echo "=========================================="
echo ""
echo "Output location: $OUTPUT_DIR"
echo "Package name: $PACKAGE_NAME"
echo "Version: $PACKAGE_VERSION"
echo ""
echo "Next steps:"
echo "  1. cd $OUTPUT_DIR"
echo "  2. npm install"
echo "  3. npm run build"
echo "  4. npm publish (to publish to npm)"
echo ""
echo "To test the SDK:"
echo "  npx ts-node example.ts"
echo ""
