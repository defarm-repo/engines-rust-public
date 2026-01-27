# IPCM Contract v2.1.0 Upgrade Summary

## 🎉 What Was Done

### 1. Contract Upgrades (by backbone AI)
The IPCM smart contract was upgraded to **v2.1.0** with two critical features:

#### ✅ Event Emission
- Contract now emits blockchain events for every CID update
- Events contain: `(dfid, cid, timestamp, updater_address)`
- **Cost**: CPU only (no storage or rent fees)
- **Retention**: 7 days via Soroban RPC
- **Purpose**: Enable cost-efficient off-chain timeline indexing

#### ✅ Upgrade Function
- Added `upgrade(new_wasm_hash)` function (admin only)
- Future contract updates won't require new addresses
- Eliminates need to update all references for future improvements

### 2. New Contract Addresses

#### Testnet (v2.1.0)
- **New**: `CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS`
- **Old**: `CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS` (deprecated)
- **Explorer**: https://stellar.expert/explorer/testnet/contract/CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS

#### Mainnet (v2.1.0)
- **New**: `CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ`
- **Old**: `CBSIAY6QWRSRPXT2I2KP7TPFDH6G3BEPL4I7PPXTAXKQHTJYE5EC4P24` (deprecated)

### 3. Updated References in Engines Codebase

All references updated in the following files:

#### Source Code
- ✅ `src/stellar_client.rs` - Contract address constants
- ✅ `.env` - Environment variables (testnet + mainnet)
- ✅ `setup-event-listener-env.sh` - Railway setup script

#### Documentation
- ✅ `docs/development/INTEGRATION_QUICKSTART.md`
- ✅ `docs/archived/STELLAR_ADAPTERS_STATUS.md`
- ✅ `docs/archived/ADAPTER_CONFIG_FIX.md`
- ✅ `docs/archived/REAL_BLOCKCHAIN_INTEGRATION.md`

#### Git Commits
- `3c48b0b` - feat: update IPCM contract addresses to v2.1.0
- `d9c5128` - docs: update IPCM contract addresses in documentation

### 4. Railway Deployment
- ✅ Code pushed to GitHub (`main` branch)
- ✅ Deployment triggered on Railway
- ✅ New contract addresses will be active after deployment completes

## 📊 Architecture Understanding

### Three Approaches to CID Timeline

1. **Diagnostic Logs** (current old contract)
   - Only available during transaction execution
   - **Not queryable** after transaction completes
   - Zero cost (not persisted)
   - ❌ Cannot be used for timeline

2. **Blockchain Events** (new v2.1.0 contract)
   - Emitted via `env.events().publish()`
   - **Queryable** via Soroban RPC for 7 days
   - **Cost**: CPU only (no storage/rent)
   - ✅ Perfect for off-chain timeline indexing
   - Enables event listener to build PostgreSQL timeline

3. **Contract Storage** (also in contract)
   - Contract stores full history in `HIST` map
   - **Queryable** via `get_history(dfid)` function
   - **Cost**: Storage + ongoing rent (most expensive)
   - ✅ Permanent timeline available on-chain
   - Can query directly from contract

### Recommended Approach
Use **both**:
- Events for efficient off-chain indexing (event listener → PostgreSQL)
- Contract storage as backup/verification source

## 🔄 Next Steps

### Immediate (After Deployment Completes)
1. **Test new contract with event emission**
   ```bash
   # Create test script that:
   # 1. Pushes item to circuit with Stellar adapter
   # 2. Queries transaction for emitted events
   # 3. Verifies event contains DFID + CID
   ```

2. **Verify Railway deployment**
   ```bash
   railway logs --service defarm-engines-api | grep -E "IPCM|Stellar"
   ```

### Short-term (Next Development Cycle)
1. **Update Event Listener** (if not already done)
   - Parse emitted events from new contract
   - Extract DFID + CID from event data
   - Store in PostgreSQL `item_cid_timeline` table

2. **Deploy Event Listener Service**
   - Ensure it's monitoring the new contract address
   - Verify it's successfully indexing events

3. **Test Complete Flow**
   - Push item → Contract emits event → Listener captures → Timeline updated

### Long-term (Future Enhancements)
1. **Monitor Event Storage**
   - Events expire after 7 days
   - Event listener must run regularly to not miss events

2. **Backup Strategy**
   - Since events expire, ensure PostgreSQL has all timeline data
   - Can always query `get_history()` from contract as fallback

## ⚠️ Important Notes

### For Other Services
If you have other services (defarm-tracker, defarm-app, etc.) that reference the IPCM contract addresses, you need to update them too:

```bash
# Search for old addresses in other repositories
grep -r "CAALVDSF7RLM7IRGE3GQKPRHWWZSPDSNHOBEIEDJU5MAM4I4PVFWJXLS" ~/defarm/
grep -r "CBSIAY6QWRSRPXT2I2KP7TPFDH6G3BEPL4I7PPXTAXKQHTJYE5EC4P24" ~/defarm/
```

### Cost Savings
The event-based approach saves significant costs:
- **Before**: Every CID stored on-chain → storage fees + ongoing rent
- **After**: Events only use CPU → no storage costs, timeline built off-chain

### Backwards Compatibility
- Old contract still works (deprecated)
- Items pushed before upgrade have timeline in contract storage
- Items pushed after upgrade have timeline via events + storage

## 📚 Reference Links

- **Stellar Events Documentation**: https://developers.stellar.org/docs/learn/fundamentals/stellar-data-structures/events
- **Soroban RPC getEvents**: https://developers.stellar.org/docs/data/rpc/api-reference/methods/getEvents
- **Event Listener Implementation**: `src/blockchain_event_listener.rs`
- **IPCM Contract Source**: `~/defarm/backbone/contracts-ipcm/src/lib.rs`

---

**Generated**: 2025-10-18
**Contract Version**: v2.1.0
**Deployed By**: Backbone AI
**Engines Updated By**: Claude (engines AI)
