# Request to Backbone AI: Add Event-Only Function to IPCM Contract

## Background

The current IPCM v2.1.0 contract emits events, which is great! However, the event emission is coupled with persistent storage writes in the `update_ipcm()` function, which incurs storage fees.

For high-frequency timeline updates where on-chain storage verification isn't required, we want to emit events WITHOUT storing data on-chain.

## Requested Change

### Add New Function: `emit_update_event`

Please add a new public function to the IPCM contract that **only emits an event** without writing to storage:

```rust
/// Emit an update event without storing data on-chain
///
/// This is useful for:
/// - High-frequency timeline updates
/// - Off-chain indexing via event listeners
/// - Cost optimization (CPU-only, no storage fees)
///
/// # Arguments
/// * `updated_by` - Address authorizing this event emission
/// * `ipcm_key` - The DFID (DeFarm ID)
/// * `cid` - The IPFS CID to associate with this DFID
pub fn emit_update_event(
    env: Env,
    updated_by: Address,
    ipcm_key: String,
    cid: String,
) {
    // Require authorization from the caller
    updated_by.require_auth();

    // Emit event ONLY (no storage write)
    env.events().publish(
        (symbol_short!("update"), ipcm_key.clone()),
        (cid.clone(), env.ledger().timestamp(), updated_by.clone()),
    );
}
```

## Benefits

1. **Cost Optimization**: CPU-only cost (~0.00001 XLM) vs storage fees (~0.0001+ XLM)
2. **Event Compatibility**: Uses same event format as `update_ipcm()`, so existing event listeners work without changes
3. **Flexibility**: Users can choose:
   - `update_ipcm()` - Store on-chain + emit event (for important records)
   - `emit_update_event()` - Only emit event (for frequent timeline updates)

## Deployment

- Target version: **IPCM v2.2.0** (or v2.1.1 if minor update)
- Networks: Both testnet and mainnet
- Use the existing upgrade function to deploy

## Contract Addresses (for reference)

- **Testnet**: `CCDJV6VAFC2MSSDSL4AEJB5BAMGDA5PMCUIZ3UF6AYIJL467PQTBZ7BS`
- **Mainnet**: `CBHYQKSG2ZADD7NXZPLFZIH7ZK766VA3YWRLISKJ6PH6KXJ4JZ52OLNZ`

## Testing

After deployment, we'll test:
1. Call `emit_update_event()` with test DFID and CID
2. Verify event is emitted with correct structure
3. Confirm event listener captures it
4. Verify PostgreSQL timeline is populated
5. Compare cost: should be ~10x cheaper than `update_ipcm()`

## Questions?

If you need any clarification or have suggestions for improvements, please let us know!

---
**Requested by**: DeFarm Engines Team
**Date**: 2025-10-19
**Priority**: Medium (cost optimization for production)
