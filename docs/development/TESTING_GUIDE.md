# Adapter Management System - Testing Guide

## Current Implementation Status

### ✅ Backend (Rust)
- **Types**: Complete adapter configuration types in `src/types.rs`
- **Storage**: Full CRUD operations in `src/storage.rs` (InMemoryStorage)
- **Business Logic**: AdapterManager in `src/adapter_manager.rs` with validation
- **API Endpoints**: 6 of 7 endpoints in `src/api/admin.rs`
- **Compilation**: ✅ Library compiles successfully

### ✅ Frontend (React/TypeScript)
- **Types**: Complete TypeScript definitions in `adapterTypes.ts`
- **API Client**: Full client in `adapterManagementApi.ts`
- **UI Components**: List manager and configuration form
- **Integration**: Added to AdminPanel as "Adapters" tab

### ⚠️ Known Issues
1. **Test Endpoint Not Implemented**: `POST /api/admin/adapters/:id/test` has an Axum async handler compilation issue
   - The functionality exists in `AdapterManager::test_adapter()` but cannot be exposed via REST API yet
   - Frontend test button will return 404

## Testing Checklist

### Backend API Testing

Run the automated test script:
```bash
cd /Users/gabrielrondon/rust/engines
./test-adapter-api.sh
```

Or test manually with curl:

#### 1. Login as Admin
```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"hen","password":"demo123"}'
```
Expected: `{"success": true, "token": "..."}"`

#### 2. List Adapters
```bash
curl -X GET http://localhost:3000/api/admin/adapters \
  -H "Authorization: Bearer YOUR_TOKEN"
```
Expected: `{"success": true, "configs": [], "count": 0}`

#### 3. Create Adapter
```bash
curl -X POST http://localhost:3000/api/admin/adapters \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Stellar",
    "description": "Test adapter",
    "adapter_type": "StellarMainnetIpfs",
    "connection_details": {
      "endpoint": "https://horizon.stellar.org",
      "auth_type": "ApiKey",
      "api_key": "test_key",
      "timeout_ms": 30000,
      "retry_attempts": 3,
      "custom_headers": {}
    }
  }'
```
Expected: `{"success": true, "message": "Adapter configuration created successfully", "config": {...}}`

#### 4. Get Single Adapter
```bash
curl -X GET http://localhost:3000/api/admin/adapters/CONFIG_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```
Expected: `{"success": true, "config": {...}}`

#### 5. Update Adapter
```bash
curl -X PUT http://localhost:3000/api/admin/adapters/CONFIG_ID \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"description": "Updated description", "is_active": true}'
```
Expected: `{"success": true, "message": "Adapter configuration updated successfully", "config": {...}}`

#### 6. Set Default Adapter
```bash
curl -X POST http://localhost:3000/api/admin/adapters/CONFIG_ID/set-default \
  -H "Authorization: Bearer YOUR_TOKEN"
```
Expected: `{"success": true, "message": "Default adapter set successfully"}`

#### 7. Delete Adapter
```bash
curl -X DELETE http://localhost:3000/api/admin/adapters/CONFIG_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```
Expected: `{"success": true, "message": "Adapter configuration deleted successfully"}`

### Frontend UI Testing

1. **Start Backend**:
   ```bash
   cd /Users/gabrielrondon/rust/engines
   cargo run --bin defarm-api
   ```

2. **Access Admin Panel**:
   - Navigate to: http://localhost:5173/admin
   - Login with: `hen` / `demo123`
   - Click on "Adapters" tab

3. **Test UI Features**:
   - [ ] Adapter list displays correctly
   - [ ] Search/filter works
   - [ ] "Create Adapter" button opens form
   - [ ] Form has all sections (Basic Info, Connection Details, Contracts)
   - [ ] Adapter type dropdown works
   - [ ] Contract sections appear only for Stellar adapters
   - [ ] Parameter mapping editor works (FromDfid, FromItem, etc.)
   - [ ] Save creates adapter successfully
   - [ ] Edit button loads existing config
   - [ ] Update saves changes
   - [ ] Set Default marks adapter with star
   - [ ] Delete removes adapter
   - [ ] Test button shows "Not implemented" (expected)

### Validation Testing

#### Name Uniqueness
```bash
# Create first adapter
curl -X POST http://localhost:3000/api/admin/adapters \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Duplicate Test", "description": "Test", "adapter_type": "LocalLocal", "connection_details": {"endpoint": "http://localhost", "auth_type": "None", "timeout_ms": 5000, "retry_attempts": 3, "custom_headers": {}}}'

# Try to create another with same name
curl -X POST http://localhost:3000/api/admin/adapters \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Duplicate Test", "description": "Test2", "adapter_type": "LocalLocal", "connection_details": {"endpoint": "http://localhost", "auth_type": "None", "timeout_ms": 5000, "retry_attempts": 3, "custom_headers": {}}}'
```
Expected: Second request returns error: `{"success": false, "error": "Adapter with name 'Duplicate Test' already exists"}`

#### Connection Validation
```bash
curl -X POST http://localhost:3000/api/admin/adapters \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Invalid", "description": "Test", "adapter_type": "LocalLocal", "connection_details": {"endpoint": "", "auth_type": "None", "timeout_ms": 0, "retry_attempts": 3, "custom_headers": {}}}'
```
Expected: Error about empty endpoint or zero timeout

#### Cannot Delete Default
```bash
# First set adapter as default
curl -X POST http://localhost:3000/api/admin/adapters/CONFIG_ID/set-default \
  -H "Authorization: Bearer YOUR_TOKEN"

# Then try to delete it
curl -X DELETE http://localhost:3000/api/admin/adapters/CONFIG_ID \
  -H "Authorization: Bearer YOUR_TOKEN"
```
Expected: Error: `{"success": false, "error": "Cannot delete the default adapter"}`

## Contract Data Structure Testing

### Mint Contract Example
Test that the frontend correctly sends and backend correctly processes:
```json
{
  "mint_contract": {
    "contract_address": "GCXYZ123...",
    "abi_url": "https://stellar.org/abi/mint.json",
    "methods": {
      "mint_dfid": {
        "method_name": "mint_dfid",
        "description": "Mints new DFID",
        "parameters": [
          {
            "param_name": "dfid",
            "param_type": "string",
            "description": "The DFID",
            "source": "FromDfid",
            "required": true
          },
          {
            "param_name": "metadata",
            "param_type": "string",
            "description": "Item metadata",
            "source": {"FromItem": "metadata"},
            "required": false
          }
        ],
        "return_type": "bool"
      }
    }
  }
}
```

### IPCM Contract Example
```json
{
  "ipcm_contract": {
    "contract_address": "GCABC456...",
    "abi_url": "https://stellar.org/abi/ipcm.json",
    "methods": {
      "record_change": {
        "method_name": "record_change",
        "description": "Records change event",
        "parameters": [
          {
            "param_name": "dfid",
            "param_type": "string",
            "source": "FromDfid",
            "required": true
          },
          {
            "param_name": "cid",
            "param_type": "string",
            "source": {"FromEvent": "cid"},
            "required": true
          }
        ],
        "return_type": "bytes32"
      }
    }
  }
}
```

## Integration Test Results

Run the automated test suite and record results:

```bash
./test-adapter-api.sh > test_results.log 2>&1
```

### Expected Output
```
==================================
Adapter Management API Test Suite
==================================

Step 1: Logging in as admin user...
✅ Login successful

Step 2: Listing existing adapters...
✅ List returned

Step 3: Creating new Stellar adapter configuration...
✅ Adapter created successfully

Step 4: Retrieving created adapter...
✅ Adapter retrieved

Step 5: Updating adapter description...
✅ Update successful

Step 6: Setting as default adapter...
✅ Default set

Step 7: Listing active adapters only...
✅ Active list returned

Step 8: Testing adapter...
⚠️  Expected to fail (not implemented)

Step 9: Deleting test adapter...
✅ Delete successful

Step 10: Verifying deletion...
✅ Adapter successfully deleted
```

## Known Frontend-Backend Contract Differences

Check these carefully during testing:

1. **ParameterSource Enum**: Frontend sends `"FromDfid"` as string, backend expects enum variant
2. **Auth Type**: Frontend sends `"None"`, `"ApiKey"`, etc. - verify serialization
3. **Timestamps**: Backend uses ISO 8601, check frontend parsing
4. **Error Messages**: Frontend should handle all backend error formats

## Recommended Test Order

1. ✅ Run automated backend test script first
2. ✅ Test frontend UI manually
3. ✅ Test validation edge cases
4. ✅ Test with different adapter types (LocalLocal, IpfsIpfs, Stellar variants)
5. ✅ Test with and without contract configs
6. ✅ Test with different auth types
7. ⚠️ Skip test endpoint (known issue)

## Success Criteria

- [ ] All API endpoints respond correctly
- [ ] Frontend can create adapters via UI
- [ ] Frontend can edit existing adapters
- [ ] Frontend can delete adapters
- [ ] Frontend can set default adapter
- [ ] Validation errors display correctly in UI
- [ ] Contract configuration saves and loads correctly
- [ ] Parameter mapping works for all source types
- [ ] Admin authentication required for all endpoints
- [ ] Non-admin users cannot access endpoints

## Next Steps After Testing

1. **If tests pass**: System is ready for production use
2. **If tests fail**: Debug and fix integration issues
3. **Future work**: Implement test endpoint once Axum async handler issue is resolved
4. **Future work**: Add unit tests for AdapterManager validation logic
5. **Future work**: Add integration tests for storage layer
