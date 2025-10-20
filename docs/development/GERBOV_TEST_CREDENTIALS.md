# Gerbov Test User - Credentials & Circuit Information

## 🔐 Gerbov Login Credentials

```
Username: gerbov
Password: Gerbov2024!Test
User ID: user-2da9af70-c4c3-4b13-9180-dc1c7094b27c
Tier: Professional
```

## 🔄 Circuit Information

### Gerbov Working Circuit (UPDATED - Use This!)

```
Circuit ID: 002ea6db-6b7b-4a69-8780-1f01ae074265
Circuit Name: Gerbov Test Circuit 1760958678
Owner: gerbov (user-2da9af70-c4c3-4b13-9180-dc1c7094b27c)
Tier: Professional
Status: ✅ VERIFIED WORKING (October 20, 2025)
```

### Circuit Configuration

- **Adapter Type:** StellarTestnetIpfs (Stellar testnet NFTs + IPFS storage)
- **Requires Approval:** No (direct push)
- **Default Namespace:** bovino
- **Allowed Namespaces:** bovino, aves, suino, soja, milho, generic
- **Blockchain:** Stellar Testnet
- **NFT Contract:** CDOZEG35YQ7KYASQBUW2DVV7CIQZB5HMWAB2PWPUCHSTKSCD5ZUTPUW3

### Alias Requirements

- **Required Canonical Identifiers:** sisbov
- **Required Contextual Identifiers:** None
- **Use Fingerprint:** No
- **Auto-apply Namespace:** Yes

### Gerbov's Permissions

As circuit owner, gerbov has **ALL** permissions:

- ✅ Push
- ✅ Pull
- ✅ Invite
- ✅ ManageMembers
- ✅ ManagePermissions
- ✅ ManageRoles
- ✅ Delete
- ✅ Certify
- ✅ Audit

## 📝 Quick Test Example

### 1. Login and Get Token

```bash
API_BASE="https://defarm-engines-api-production.up.railway.app/api"

RESPONSE=$(curl -s -X POST "$API_BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"gerbov","password":"Gerbov2024!Test"}')

TOKEN=$(echo "$RESPONSE" | jq -r '.token')
echo "Token: $TOKEN"
```

### 2. Create Local Item

```bash
LOCAL_RESPONSE=$(curl -s -X POST "$API_BASE/items/local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "enhanced_identifiers": [
      {
        "namespace": "bovino",
        "key": "sisbov",
        "value": "BR001122334455",
        "id_type": "Canonical"
      }
    ],
    "enriched_data": {
      "breed": "Nelore",
      "birth_date": "2024-01-15",
      "weight_kg": 450
    }
  }')

LOCAL_ID=$(echo "$LOCAL_RESPONSE" | jq -r '.local_id')
echo "Local ID: $LOCAL_ID"
```

### 3. Push to Circuit (Tokenization + Blockchain)

```bash
CIRCUIT_ID="002ea6db-6b7b-4a69-8780-1f01ae074265"

PUSH_RESPONSE=$(curl -s -X POST "$API_BASE/circuits/$CIRCUIT_ID/push-local" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"local_id\": \"$LOCAL_ID\",
    \"identifiers\": [
      {
        \"namespace\": \"bovino\",
        \"key\": \"sisbov\",
        \"value\": \"BR001122334455\",
        \"id_type\": \"Canonical\"
      }
    ]
  }")

echo "$PUSH_RESPONSE" | jq '.'
```

### 4. View Storage History (Blockchain TXs)

```bash
DFID=$(echo "$PUSH_RESPONSE" | jq -r '.dfid')

curl -s "$API_BASE/items/$DFID/storage-history" \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

## 🔗 Useful API Endpoints

All endpoints use base URL: `https://defarm-engines-api-production.up.railway.app/api`

- **POST /auth/login** - Get JWT token
- **POST /items/local** - Create local item
- **POST /circuits/{circuit_id}/push-local** - Push to circuit
- **GET /items/{dfid}/storage-history** - View blockchain transactions
- **GET /circuits/{circuit_id}** - View circuit details
- **GET /items/local/duplicates** - Find duplicate local items
- **POST /items/local/organize** - Batch merge duplicates
- **POST /items/local/unmerge** - Undo merge operation

## 📊 Testing Workflow

1. ✅ Login as gerbov
2. ✅ Create one or more local items (offline)
3. ✅ Use organization endpoints to detect/merge duplicates (optional)
4. ✅ Push to circuit for tokenization
5. ✅ View storage history to see Stellar testnet transactions
6. ✅ Verify IPFS CID and blockchain TX hash

## 🎯 Integration Testing

This user and circuit are ready for:

- API integration testing
- SDK development
- Documentation examples
- Client demonstrations
- Webhook testing
- Full end-to-end workflows

---

**Last Updated:** 2025-10-20
**Environment:** Production Railway (defarm-engines-api-production.up.railway.app)
**Status:** ✅ Active and Ready for Testing
