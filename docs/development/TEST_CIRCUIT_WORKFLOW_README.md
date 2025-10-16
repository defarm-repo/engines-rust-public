# Comprehensive Circuit Workflow Integration Test

## Overview

This test suite validates the complete end-to-end workflow of the DeFarm circuit system, including:
- Circuit creation and configuration
- User authentication and join workflows
- Item creation, tokenization, and pushing
- All adapter types (Local, IPFS, Stellar)
- Storage history and migration
- Encrypted event visibility
- Auto-approval vs manual approval workflows

## Prerequisites

### 1. Environment Setup

Ensure you have the following environment variables set:

```bash
export JWT_SECRET="defarm-dev-secret-key-minimum-32-characters-long-2024"

# Optional: IPFS/Pinata credentials (for IPFS adapter tests)
export PINATA_API_KEY="your-pinata-api-key"
export PINATA_SECRET_KEY="your-pinata-secret-key"
export PINATA_JWT="your-pinata-jwt"

# Optional: Stellar credentials (for Stellar adapter tests)
export STELLAR_TESTNET_IPCM_CONTRACT="your-testnet-contract-address"
export STELLAR_MAINNET_IPCM_CONTRACT="your-mainnet-contract-address"
```

### 2. Dependencies

The test script requires:
- `curl` - for making HTTP requests
- `jq` - for JSON parsing
- `bash` 4.0+ - for associative arrays

Install dependencies:
```bash
# macOS
brew install jq

# Ubuntu/Debian
sudo apt-get install jq curl

# Fedora/RHEL
sudo dnf install jq curl
```

### 3. API Server

Start the DeFarm API server:
```bash
cd /Users/gabrielrondon/rust/engines
cargo run --bin defarm-api
```

The API should be running at `http://localhost:3000`

## Running the Tests

### Full Test Suite

Run all scenarios:
```bash
./test-circuit-workflow.sh
```

### Specific Scenarios

The script is modular. You can comment out scenarios in the `main()` function to run specific tests.

## Test Scenarios

### ✅ Scenario 1: Circuit Setup - Auto-Approve Members ON, Auto-Publish Items ON

**Purpose**: Test circuit creation with automatic member approval and item publishing.

**Tests**:
- Admin creates circuit with default settings
- Configure public settings with auto-approve and auto-publish enabled
- Set adapter to LocalLocal
- Configure webhook for ItemPushed events

**Assertions**:
- Circuit created successfully
- Public access mode set correctly
- Auto-approve and auto-publish flags are enabled
- Adapter configuration applied

---

### ✅ Scenario 2: User Auto-Join and Push

**Purpose**: Verify users can automatically join public circuits and push items.

**Tests**:
- User authenticates and joins public circuit
- User is auto-approved (no manual intervention)
- User creates local item with basic data (~1KB)
- User pushes item to circuit
- Item is auto-published to circuit public items
- Storage history is recorded for LocalLocal adapter

**Assertions**:
- User auto-approved without manual approval
- User is member of circuit
- Local item created with LID
- Item tokenized with DFID assigned
- Item status is "NewItemCreated"
- Item auto-published to circuit
- Storage record shows LocalLocal adapter
- Storage record is active

---

### ✅ Scenario 3: Circuit Setup - Manual Approval Required

**Purpose**: Test circuit with manual approval requirements for joins and pushes.

**Tests**:
- Admin creates circuit requiring manual approvals
- Set `require_approval_for_push: true`
- Set `auto_approve_members: false`
- Set adapter to IpfsIpfs
- User2 requests to join circuit
- Join request is pending
- Admin approves join request

**Assertions**:
- Circuit configured to require push approval
- Auto-approve members is disabled
- IpfsIpfs adapter set correctly
- Join request created and pending
- Admin can list pending requests
- User2 becomes member after approval

---

### ✅ Scenario 4: Push with Manual Approval

**Purpose**: Verify push operations requiring manual approval.

**Tests**:
- User2 creates item with medium data (~100KB, 10 identifiers, 50 fields)
- User2 pushes item to circuit
- Operation is pending approval
- Admin lists and approves operation
- DFID is assigned after approval
- Storage history shows IPFS CID

**Assertions**:
- Medium-sized item created successfully
- Push operation is pending
- Admin can list pending operations
- Operation approved successfully
- DFID assigned after approval
- Storage shows IpfsIpfs adapter
- Storage location contains CID

---

### ✅ Scenario 5: Large Data Test

**Purpose**: Test handling of large items (~1MB).

**Tests**:
- Create item with 100 identifiers
- Add large text content (~100KB base64)
- Include nested data structures
- Push to circuit with IpfsIpfs adapter
- Verify CID format and storage metadata

**Assertions**:
- Large item created successfully
- Item tokenized correctly
- CID format is valid (Qm... or baf...)
- Storage metadata includes hash
- Large data handled without errors

---

### ✅ Scenario 6: Test All Adapters

**Purpose**: Verify all 6 adapter types work correctly.

**Tests**: For each adapter (LocalLocal, IpfsIpfs, LocalIpfs, StellarTestnetIpfs, StellarMainnetIpfs, StellarMainnetStellarMainnet):
- Create circuit with specific adapter
- Add user as member
- Create and push item
- Query storage history
- Verify adapter-specific storage location fields

**Assertions** (per adapter):
- Circuit created successfully
- Item tokenized with adapter
- Storage record shows correct adapter type
- Storage location has correct fields:
  - LocalLocal/LocalIpfs: `id` field
  - IpfsIpfs: `cid` field
  - Stellar adapters: `transaction_id` field
- Storage metadata includes hash (BLAKE3)

---

### ✅ Scenario 7: Storage Migration Test

**Purpose**: Test automatic storage migration when circuit adapter changes.

**Tests**:
- Create circuit with LocalLocal adapter
- Push item (stored locally)
- Change circuit adapter to IpfsIpfs
- Update item to trigger migration
- Verify multiple storage records
- Check old record is inactive, new record is active

**Assertions**:
- Item initially stored with LocalLocal
- One storage record before migration
- Adapter change successful
- After update, multiple storage records exist
- Old LocalLocal record is inactive
- New IpfsIpfs record is active
- Storage history properly tracks migration

---

### ✅ Scenario 8: Encrypted Events Test

**Purpose**: Verify encrypted event visibility settings.

**Tests**:
- Create circuit with `show_encrypted_events: true`
- Push item and verify encrypted events are visible in public info
- Create circuit with `show_encrypted_events: false`
- Verify encrypted events are NOT visible in public info

**Assertions**:
- Public settings correctly show encrypted events enabled/disabled
- Public circuit info respects encryption visibility settings

---

## Test Output

The test script provides colored, detailed output:

- 🔵 **Blue**: Section headers
- 🟡 **Yellow**: Sub-sections
- ✅ **Green**: Passed tests
- ❌ **Red**: Failed tests
- ℹ️ **White**: Informational messages

### Example Output

```bash
========================================
SCENARIO 1: Circuit Setup - Auto-Approve Members ON, Auto-Publish Items ON
========================================

>>> Authenticating as hen-admin-001
✅ Authentication successful
>>> Creating circuit with default settings
✅ Circuit created successfully
   Circuit ID: 550e8400-e29b-41d4-a716-446655440000
>>> Configuring public settings (auto-approve: true, auto-publish: true)
✅ Circuit access mode set to Public
✅ Auto-approve members enabled
✅ Auto-publish items enabled
...
```

### Final Summary

```bash
========================================
TEST SUMMARY
========================================
Total Tests: 87
Passed: 87
Failed: 0

✅ ALL TESTS PASSED!
```

## Exit Codes

- `0`: All tests passed
- `1`: One or more tests failed or API server not running

## Troubleshooting

### API Server Not Running

```
❌ API server is not running at http://localhost:3000/api
Please start the API server with: cargo run --bin defarm-api
```

**Solution**: Start the API server before running tests.

### Authentication Failures

```
❌ Authentication failed
```

**Solution**:
- Ensure `JWT_SECRET` is set correctly
- Verify user credentials exist in development data
- Check the API server logs for errors

### IPFS/Stellar Tests Failing

**Solution**:
- For IPFS tests: Ensure Pinata credentials are set
- For Stellar tests: Ensure contract addresses are configured
- Tests gracefully handle missing credentials by continuing

### jq Not Found

```
bash: jq: command not found
```

**Solution**: Install `jq` package (see Prerequisites section).

## Extending the Tests

### Adding New Scenarios

1. Create a new function following the pattern:

```bash
scenario_X_your_test_name() {
    print_header "SCENARIO X: Your Test Name"

    # Your test logic here
    assert_not_empty "$value" "Description of what is tested"

    print_success "Scenario X completed"
}
```

2. Add the function call in `main()`:

```bash
main() {
    # ...
    scenario_8_encrypted_events
    scenario_X_your_test_name  # Add here
    # ...
}
```

### Adding Custom Assertions

Create helper functions for common assertions:

```bash
assert_http_status() {
    local response="$1"
    local expected_status="$2"
    local actual_status=$(echo "$response" | jq -r '.status')
    assert_equals "$expected_status" "$actual_status" "HTTP status"
}
```

## Future Enhancements

Scenarios planned but not yet implemented:

- **Scenario 9**: Webhook/Post-Action Comprehensive Test
- **Scenario 10**: Batch Operations Test
- **Scenario 11**: Item Updates and History
- **Scenario 12**: Pending Items Approval

These can be added following the same pattern as scenarios 1-8.

## Notes

- Tests create real data in the running instance
- Each test scenario is independent but may build on previous scenarios
- Test data is ephemeral (in-memory storage)
- Restart the API server between test runs for a clean state
- Some tests include `sleep` commands to allow for async operations (IPFS, Stellar)

## Support

For issues or questions:
- Check the API server logs: `cargo run --bin defarm-api`
- Review the test output for specific failures
- Ensure all prerequisites are met
- Verify environment variables are set correctly

## License

This test suite is part of the DeFarm engine project.
