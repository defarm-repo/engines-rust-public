#!/bin/bash
# DeFarm Engines - Python SDK Generator
# This script generates a Python SDK from the OpenAPI specification

set -e

echo "🐍 DeFarm Engines Python SDK Generator"
echo "========================================"
echo ""

# Configuration
OPENAPI_FILE="../docs/api/openapi.yaml"
OUTPUT_DIR="./python"
PACKAGE_NAME="defarm-sdk"
PACKAGE_VERSION="1.0.0"

# Check if OpenAPI file exists
if [ ! -f "$OPENAPI_FILE" ]; then
    echo "❌ Error: OpenAPI file not found at $OPENAPI_FILE"
    exit 1
fi

echo "✓ OpenAPI specification found"

# Check if docker or python is available
if command -v docker &> /dev/null; then
    echo "✓ Docker detected - will use openapi-generator container"
    USE_DOCKER=true
elif command -v python3 &> /dev/null; then
    echo "✓ Python detected - will use local generator"
    USE_DOCKER=false
else
    echo "❌ Error: Neither Docker nor Python found"
    exit 1
fi

# Clean output directory
if [ -d "$OUTPUT_DIR" ]; then
    echo "🧹 Cleaning existing output directory..."
    rm -rf "$OUTPUT_DIR"
fi

mkdir -p "$OUTPUT_DIR"
echo "✓ Output directory prepared: $OUTPUT_DIR"

# Generate Python SDK
echo ""
echo "📦 Generating Python SDK..."
echo ""

if [ "$USE_DOCKER" = true ]; then
    docker run --rm \
        -v "${PWD}/../docs/api:/input" \
        -v "${PWD}/${OUTPUT_DIR}:/output" \
        openapitools/openapi-generator-cli generate \
        -i /input/openapi.yaml \
        -g python \
        -o /output \
        --package-name defarm \
        --additional-properties=projectName=defarm-sdk,packageVersion=${PACKAGE_VERSION}
else
    pip3 install openapi-generator-cli 2>/dev/null || true
    openapi-generator-cli generate \
        -i "$OPENAPI_FILE" \
        -g python \
        -o "$OUTPUT_DIR" \
        --package-name defarm \
        --additional-properties=projectName=defarm-sdk,packageVersion=${PACKAGE_VERSION}
fi

echo ""
echo "✓ SDK generation complete!"

# Create enhanced README
echo ""
echo "📖 Creating enhanced README.md..."

cat > "$OUTPUT_DIR/README.md" << 'EOF'
# DeFarm Engines Python SDK

Official Python SDK for the DeFarm Engines API.

## Installation

```bash
pip install defarm-sdk
```

## Quick Start

```python
from defarm import ApiClient, Configuration
from defarm.api import AuthApi, CircuitsApi, ItemsApi

# Configure API client
config = Configuration(
    host="https://connect.defarm.net"
)

# Login
with ApiClient(config) as api_client:
    auth_api = AuthApi(api_client)

    # Get JWT token
    login_response = auth_api.login({
        "username": "your_username",
        "password": "your_password"
    })

    token = login_response.token

    # Update configuration with token
    config.access_token = token

    # Create new client with token
    with ApiClient(config) as authenticated_client:
        items_api = ItemsApi(authenticated_client)
        circuits_api = CircuitsApi(authenticated_client)

        # Create local item
        item = items_api.create_local_item({
            "identifiers": [{
                "namespace": "bovino",
                "key": "sisbov",
                "value": "BR12345678901234",
                "id_type": "Canonical",
                "verified": False
            }],
            "enriched_data": {
                "weight": "500kg",
                "breed": "Angus"
            }
        })

        print(f"Item created with LID: {item.local_id}")

        # List circuits
        circuits = circuits_api.list_circuits()
        print(f"Found {len(circuits)} circuits")

        # Push to circuit
        if circuits:
            circuit_id = circuits[0].circuit_id
            result = circuits_api.push_local_item_to_circuit(
                circuit_id=circuit_id,
                body={"local_id": item.local_id}
            )
            print(f"Item tokenized with DFID: {result.dfid}")
```

## Using API Keys

```python
from defarm import ApiClient, Configuration
from defarm.api import ItemsApi

# Configure with API key
config = Configuration(
    host="https://connect.defarm.net",
    api_key={"X-API-Key": "dfm_your_api_key"}
)

with ApiClient(config) as api_client:
    items_api = ItemsApi(api_client)
    items = items_api.list_items()
    print(f"Found {len(items)} items")
```

## Features

- ✅ Fully typed with Python type hints
- ✅ Auto-generated from OpenAPI specification
- ✅ Supports both JWT and API Key authentication
- ✅ Async/await support (optional)
- ✅ Comprehensive error handling
- ✅ Built on urllib3 for HTTP requests

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

## Error Handling

```python
from defarm.exceptions import ApiException

try:
    items = items_api.list_items()
except ApiException as e:
    print(f"API Error: {e.status} - {e.reason}")
    print(f"Response body: {e.body}")
```

## Documentation

Full API documentation: https://connect.defarm.net/docs

## Support

- Issues: https://github.com/defarm/sdk-python/issues
- Email: support@defarm.net

## License

MIT
EOF

echo "✓ README.md created"

# Create example script
echo ""
echo "📝 Creating example..."

cat > "$OUTPUT_DIR/example.py" << 'EOF'
#!/usr/bin/env python3
"""
DeFarm SDK Example Usage
This example demonstrates basic SDK operations
"""

from defarm import ApiClient, Configuration
from defarm.api import AuthApi, CircuitsApi, ItemsApi, EventsApi
from defarm.exceptions import ApiException
import time

def main():
    # Configure API client
    config = Configuration(
        host="https://connect.defarm.net"
    )

    try:
        # Step 1: Login
        print("Logging in...")
        with ApiClient(config) as api_client:
            auth_api = AuthApi(api_client)
            login_response = auth_api.login({
                "username": "hen",
                "password": "demo123"
            })

            token = login_response.token
            user_id = login_response.user_id
            print(f"✓ Logged in as: {user_id}")

        # Step 2: Create authenticated client
        config.access_token = token

        with ApiClient(config) as authenticated_client:
            items_api = ItemsApi(authenticated_client)
            circuits_api = CircuitsApi(authenticated_client)
            events_api = EventsApi(authenticated_client)

            # Step 3: Create local item
            print("\nCreating local item...")
            item = items_api.create_local_item({
                "identifiers": [{
                    "namespace": "test",
                    "key": "example",
                    "value": f"SDK-{int(time.time())}",
                    "id_type": "Contextual",
                    "verified": False
                }],
                "enriched_data": {
                    "source": "python-sdk-example",
                    "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ")
                }
            })
            local_id = item.local_id
            print(f"✓ Item created with LID: {local_id}")

            # Step 4: List circuits
            print("\nListing circuits...")
            circuits = circuits_api.list_circuits()
            print(f"✓ Found {len(circuits)} circuits")

            if circuits:
                circuit = circuits[0]
                print(f"  First circuit: {circuit.name}")

                # Step 5: Push to circuit
                print("\nPushing item to circuit...")
                result = circuits_api.push_local_item_to_circuit(
                    circuit_id=circuit.circuit_id,
                    body={"local_id": local_id}
                )
                dfid = result.dfid
                print(f"✓ Item tokenized with DFID: {dfid}")

                # Step 6: Create event
                print("\nCreating event...")
                event = events_api.create_event({
                    "dfid": dfid,
                    "event_type": "Enriched",
                    "visibility": "Public",
                    "metadata": {
                        "action": "sdk_test",
                        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ")
                    }
                })
                print(f"✓ Event created: {event.event_id}")

                # Step 7: Get item details
                print("\nFetching item details...")
                item_details = items_api.get_item(dfid=dfid)
                print(f"✓ Item has {len(item_details.identifiers)} identifiers")

            print("\n✅ Example completed successfully!")

    except ApiException as e:
        print(f"❌ API Error: {e.status} - {e.reason}")
        if e.body:
            print(f"Response: {e.body}")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    main()
EOF

chmod +x "$OUTPUT_DIR/example.py"
echo "✓ example.py created"

# Create setup.py enhancement
echo ""
echo "⚙️  Enhancing setup.py..."

if [ -f "$OUTPUT_DIR/setup.py" ]; then
    # Add development dependencies
    cat >> "$OUTPUT_DIR/setup.py" << 'EOF'

# Development dependencies
extras_require = {
    'dev': [
        'pytest>=7.0.0',
        'pytest-cov>=4.0.0',
        'black>=23.0.0',
        'mypy>=1.0.0',
        'flake8>=6.0.0'
    ]
}
EOF
    echo "✓ setup.py enhanced"
fi

# Summary
echo ""
echo "========================================"
echo "✅ Python SDK Generated Successfully!"
echo "========================================"
echo ""
echo "Output location: $OUTPUT_DIR"
echo "Package name: $PACKAGE_NAME"
echo "Version: $PACKAGE_VERSION"
echo ""
echo "Next steps:"
echo "  1. cd $OUTPUT_DIR"
echo "  2. pip install -e ."
echo "  3. python example.py (to test)"
echo "  4. python setup.py sdist bdist_wheel (to build)"
echo "  5. twine upload dist/* (to publish to PyPI)"
echo ""
echo "To install development dependencies:"
echo "  pip install -e .[dev]"
echo ""
