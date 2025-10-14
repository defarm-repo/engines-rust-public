# Client Demo Setup - Ready to Share

## ✅ Everything is Configured and Running!

The API documentation is ready for your client with pre-configured testing credentials and circuit.

---

## 🌐 Documentation URL

**Send this link to your client:**
```
https://defarm-engines-api-production.up.railway.app/docs/
```

Or (when DNS is configured):
```
https://connect.defarm.net/docs/
```

---

## 👤 Client Testing Credentials

### Regular User (for your client to test)
```
Username: gerbov
Password: Gerbov2024!Test
Tier: Professional
Access: StellarTestnetIpfs + IPFS-IPFS
```

**Your client uses these credentials to:**
- Login and get JWT token
- Create local items
- Push items to the pre-configured circuit
- View blockchain transactions

---

## 🔧 Circuit Owner (for your administration)

### Admin User (you control this)
```
Username: ms_admin
Password: MSAdmin2024!@Secure#789
Role: Admin + Circuit Owner
```

**You use these credentials to:**
- Manage the circuit
- Approve/reject members
- Configure circuit settings
- Monitor usage

---

## 🎯 Pre-Configured Circuit

### MS Rastreabilidade
```
Circuit ID: 2b2fbb90-2850-4424-a0c2-a286b16fc6ee
Name: MS Rastreabilidade
Visibility: Public (discoverable)
Adapter: StellarTestnetIpfs (sponsored)
Network: Stellar Testnet + IPFS (Pinata)
Members: gerbov (approved)
```

**Circuit Configuration:**
- ✅ Adapter sponsorship enabled (gerbov doesn't need his own adapter credentials)
- ✅ Public visibility (discoverable by others)
- ✅ Requires SISBOV identifier for cattle
- ✅ Auto-publish enabled
- ✅ gerbov is approved member

---

## 📖 What Your Client Will See

### 1. Documentation Homepage
- Beautiful Swagger UI interface
- Quick start banner with their credentials
- Pre-filled circuit ID in examples
- All 16 endpoints visible and testable

### 2. Quick Start Instructions
The banner shows:
- **Step 1:** Login with `gerbov` / `Gerbov2024!Test`
- **Step 2:** Get JWT token
- **Step 3:** Authorize
- **Step 4:** Create local item with SISBOV
- **Step 5:** Push to circuit `2b2fbb90-2850-4424-a0c2-a286b16fc6ee`
- **Step 6:** View blockchain transactions

### 3. Working Example Flow

**Client can test this immediately:**

```bash
# 1. Login
POST /api/auth/login
{
  "username": "gerbov",
  "password": "Gerbov2024!Test"
}
# Returns JWT token

# 2. Create local item
POST /api/items/local
Authorization: Bearer <token>
{
  "enhanced_identifiers": [{
    "namespace": "bovino",
    "key": "sisbov",
    "value": "BR921180523565",
    "id_type": "Canonical"
  }],
  "enriched_data": {
    "breed": "Nelore",
    "weight_kg": 450
  }
}
# Returns local_id

# 3. Push to circuit (tokenization)
POST /api/circuits/2b2fbb90-2850-4424-a0c2-a286b16fc6ee/push-local
Authorization: Bearer <token>
{
  "local_id": "<local_id_from_step_2>",
  "requester_id": "<user_id_from_login>"
}
# Returns DFID

# 4. View blockchain transactions
GET /api/items/<DFID>/storage-history
Authorization: Bearer <token>
# Returns NFT TX, IPCM TX, IPFS CID
```

---

## 🎁 What Makes This Easy for Client

### They DON'T Need To:
- ❌ Create their own user account (already created)
- ❌ Request adapter access (already granted)
- ❌ Create a circuit (already created)
- ❌ Request to join circuit (already approved)
- ❌ Configure blockchain settings (pre-configured)
- ❌ Set up IPFS credentials (sponsored by circuit)

### They ONLY Need To:
- ✅ Open the documentation URL
- ✅ Use the provided credentials (gerbov)
- ✅ Follow the 6-step quick start guide
- ✅ Test the API with their own data

---

## 🔐 Security Notes

### What Client Can Do:
- ✅ Create items in their workspace
- ✅ Push items to MS Rastreabilidade circuit
- ✅ View their own items
- ✅ View items in circuits they're member of
- ✅ Pull items from circuits (if permitted)

### What Client CANNOT Do:
- ❌ Access other users' private items
- ❌ Modify circuit settings (only ms_admin can)
- ❌ Delete the circuit
- ❌ Access admin endpoints
- ❌ See other users' workspaces

---

## 📊 Monitoring Client Activity

### As ms_admin, you can:

1. **View circuit activities**
```bash
GET /api/circuits/2b2fbb90-2850-4424-a0c2-a286b16fc6ee/activities
Authorization: Bearer <ms_admin_token>
```

2. **View all circuit items**
```bash
GET /api/circuits/2b2fbb90-2850-4424-a0c2-a286b16fc6ee/items
Authorization: Bearer <ms_admin_token>
```

3. **View circuit members**
```bash
GET /api/circuits/2b2fbb90-2850-4424-a0c2-a286b16fc6ee
Authorization: Bearer <ms_admin_token>
```

---

## 🚀 What Happens When Client Tests

### Successful Flow:
1. **Client logs in** → Gets JWT token (expires in 24 hours)
2. **Client creates item** → Gets LID (Local ID)
3. **Client pushes to circuit** → System:
   - Uploads to IPFS via Pinata
   - Mints NFT on Stellar Testnet
   - Updates IPCM contract
   - Returns DFID
4. **Client views storage** → Gets:
   - NFT transaction hash (viewable on Stellar testnet explorer)
   - IPCM transaction hash
   - IPFS CID (viewable on IPFS gateway)
   - Full storage history

### Example Real Output:
```json
{
  "success": true,
  "dfid": "DFID-20251014-000001-XXXX",
  "records": [{
    "nft_mint_tx": "a1b2c3...",
    "ipcm_update_tx": "d4e5f6...",
    "ipfs_cid": "QmXxx...",
    "network": "stellar-testnet",
    "stored_at": "2025-10-14T10:30:00Z"
  }]
}
```

---

## 📞 Support

### If Client Has Issues:

1. **Authentication Problems**
   - Verify username/password exactly as shown
   - Check token expiration (24 hours)
   - Use `/api/auth/refresh` to renew token

2. **Push Failures**
   - Verify they're using the correct circuit ID
   - Ensure at least one SISBOV identifier
   - Check they're using their user_id as requester_id

3. **Can't See Blockchain TXs**
   - Verify DFID is correct
   - Check storage history endpoint with correct DFID
   - Wait a moment after push (blockchain finalization)

---

## 🎯 Next Steps

### For Client Testing Today:
1. ✅ Send them the documentation URL
2. ✅ They login as gerbov
3. ✅ They test the complete flow
4. ✅ They see blockchain transactions

### For Production Later:
1. Create their own production user
2. Upgrade to appropriate tier
3. Create their own circuits
4. Configure custom webhooks
5. Integrate with their systems using provided SDKs

---

## 📝 What to Tell Your Client

```
Hi,

I've set up everything for you to test our blockchain tokenization API immediately.

Documentation: https://defarm-engines-api-production.up.railway.app/docs/

Login with:
- Username: gerbov
- Password: Gerbov2024!Test

The documentation has a step-by-step guide that will walk you through:
1. Authentication
2. Creating an item (e.g., cattle with SISBOV)
3. Tokenizing it on blockchain (Stellar Testnet)
4. Viewing the blockchain transaction hashes

Everything is pre-configured - you just need to follow the Quick Start guide
on the page and you'll see your data on the blockchain in minutes!

The circuit is called "MS Rastreabilidade" and it's already set up for you.

Let me know if you have any questions!
```

---

**Last Updated:** October 14, 2025
**Status:** ✅ Ready for client testing
**Environment:** Railway Production (Stellar Testnet + IPFS)
