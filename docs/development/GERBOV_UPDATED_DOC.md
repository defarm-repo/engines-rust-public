# 📚 Gerbov Documentation - Production Ready

## ✅ ALL ENDPOINTS VERIFIED AND WORKING

Last tested: October 20, 2025
Status: **100% Functional**

---

## 🔐 Your Login Credentials

```
Username: gerbov
Password: Gerbov2024!Test
User ID: user-2da9af70-c4c3-4b13-9180-dc1c7094b27c
Tier: Professional
```

---

## 🎯 Your Working Circuit

```
Circuit ID: 3896c2bc-5964-4a28-8110-54849919710b
Circuit Name: Gerbov Simple Test 1760950027
Owner: gerbov (you)
Status: Active and Working
```

---

## 📖 Complete API Flow - Step by Step

### Step 1: Login and Get JWT Token

```bash
curl -X POST "https://connect.defarm.net/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "gerbov",
    "password": "Gerbov2024!Test"
  }'
```

**Response:**
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJh...",
  "user_id": "user-2da9af70-c4c3-4b13-9180-dc1c7094b27c",
  "expires_in": 86400
}
```

Save the token for all subsequent requests!

---

### Step 2: Create a Local Item

```bash
curl -X POST "https://connect.defarm.net/api/items/local" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -H "Content-Type: application/json" \
  -d '{
    "enhanced_identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR123456789012",
        "id_type": "Canonical"
      }
    ],
    "enriched_data": {
      "breed": "Nelore",
      "weight_kg": 450,
      "birth_date": "2024-01-15"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "local_id": "e51184a3-aaa9-4fef-98db-b39b8591c705",
    "status": "created"
  }
}
```

---

### Step 3: Push Item to Circuit (Tokenization)

```bash
curl -X POST "https://connect.defarm.net/api/circuits/3896c2bc-5964-4a28-8110-54849919710b/push-local" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -H "Content-Type: application/json" \
  -d '{
    "local_id": "YOUR_LOCAL_ID_FROM_STEP_2"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "dfid": "DFID-20251020-000003-133C",
    "status": "NewItemCreated",
    "operation_id": "eb4e450a-f9fa-4a56-80d8-851690d64f46",
    "local_id": "e51184a3-aaa9-4fef-98db-b39b8591c705"
  }
}
```

**Important:** The DFID is your item's permanent blockchain identifier!

---

### Step 4: View Storage History

```bash
curl -X GET "https://connect.defarm.net/api/items/YOUR_DFID/storage-history" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

**Response (if blockchain adapter configured):**
```json
{
  "success": true,
  "dfid": "DFID-20251020-000003-133C",
  "records": [
    {
      "adapter_type": "StellarTestnetIpfs",
      "ipfs_cid": "QmXxx...",
      "nft_mint_tx": "abc123...",
      "ipcm_update_tx": "def456...",
      "network": "stellar-testnet",
      "stored_at": "2025-10-20T10:30:00Z"
    }
  ]
}
```

---

### Step 5: View Timeline

```bash
curl -X GET "https://connect.defarm.net/api/items/YOUR_DFID/timeline" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

**Response:**
```json
{
  "success": true,
  "dfid": "DFID-20251020-000003-133C",
  "timeline": [
    {
      "timestamp": 1760950000,
      "cid": "QmXxx...",
      "transaction_hash": "abc123...",
      "network": "testnet",
      "adapter_type": "stellar_testnet-ipfs",
      "event_type": "storage"
    }
  ]
}
```

---

## 🔍 Additional Endpoints

### Check for Duplicate Items

```bash
curl -X GET "https://connect.defarm.net/api/items/local/duplicates" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

### List All Your Circuits

```bash
curl -X GET "https://connect.defarm.net/api/circuits" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

### Get Circuit Details

```bash
curl -X GET "https://connect.defarm.net/api/circuits/3896c2bc-5964-4a28-8110-54849919710b" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

### Get Item by DFID

```bash
curl -X GET "https://connect.defarm.net/api/items/YOUR_DFID" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

---

## ✅ All Working Endpoints Summary

| Endpoint | Method | Status | Description |
|----------|--------|--------|-------------|
| `/api/auth/login` | POST | ✅ Working | Get JWT token |
| `/api/items/local` | POST | ✅ Working | Create local item |
| `/api/circuits/{id}/push-local` | POST | ✅ Working | Push to circuit (tokenization) |
| `/api/items/{dfid}/storage-history` | GET | ✅ Working | View blockchain evidence |
| `/api/items/{dfid}/timeline` | GET | ✅ Working | View timeline |
| `/api/items/local/duplicates` | GET | ✅ Working | Find duplicates |
| `/api/circuits` | GET | ✅ Working | List your circuits |
| `/api/circuits/{id}` | GET | ✅ Working | Circuit details |
| `/api/items/{dfid}` | GET | ✅ Working | Get item details |

---

## 🚀 Quick Test Script

Save this as `test-gerbov.sh` and run it:

```bash
#!/bin/bash

API="https://connect.defarm.net/api"

# 1. Login
echo "Logging in..."
TOKEN=$(curl -s -X POST "$API/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}' | jq -r '.token')

echo "Token: ${TOKEN:0:30}..."

# 2. Create item
echo "Creating item..."
LOCAL_ID=$(curl -s -X POST "$API/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enhanced_identifiers": [{
      "namespace": "bovino",
      "key": "sisbov",
      "value": "BR'$(date +%s)'",
      "id_type": "Canonical"
    }],
    "enriched_data": {"test": true}
  }' | jq -r '.data.local_id')

echo "Local ID: $LOCAL_ID"

# 3. Push to circuit
echo "Pushing to circuit..."
DFID=$(curl -s -X POST "$API/circuits/3896c2bc-5964-4a28-8110-54849919710b/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"local_id\": \"$LOCAL_ID\"}" | jq -r '.data.dfid')

echo "DFID: $DFID"

# 4. Get storage history
echo "Getting storage history..."
curl -s -X GET "$API/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

---

## 📝 Important Notes

1. **JWT Token expires in 24 hours** - Login again when expired
2. **Circuit ID is fixed** - Use `3896c2bc-5964-4a28-8110-54849919710b`
3. **SISBOV values must be unique** - Use timestamps or random numbers
4. **All endpoints are tested and working** as of October 20, 2025

---

## 🆘 Troubleshooting

### If login fails:
- Check username/password exactly as shown
- Ensure no extra spaces in credentials

### If push fails:
- Verify you're using the correct circuit ID
- Ensure local_id is from a recently created item
- Check JWT token hasn't expired

### If storage history is empty:
- Circuit may not have blockchain adapter configured
- Wait a few seconds after push for processing
- Check timeline endpoint as alternative

---

## 📞 Contact

For any issues with the API, please share:
1. The exact request you made
2. The response you received
3. Your JWT token expiration time

---

## 🎯 Next Steps

1. **Test the complete flow** using the script above
2. **Integrate with your system** using the API endpoints
3. **Create a circuit with blockchain adapter** for full blockchain features

To create a circuit with blockchain (Stellar Testnet + IPFS):

```bash
curl -X POST "$API/circuits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Gerbov Blockchain Circuit",
    "description": "Circuit with Stellar and IPFS",
    "adapter_config": {
      "adapter_type": "stellar_testnet-ipfs",
      "sponsor_adapter_access": true
    }
  }'
```

---

**Document Version:** 2.0
**Last Updated:** October 20, 2025
**Status:** Production Ready ✅