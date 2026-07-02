# infrix-types

The core, dependency-light type layer shared across the Infrix Rust SDK: contract
ABI types, primitive numerics, the error type, and the canonical governance
vocabulary. `#![no_std]`-friendly so it links into `wasm32-unknown-unknown`
contracts. Normally consumed transitively via [`infrix-sdk`](../infrix-sdk).

```toml
[dependencies]
infrix-types = "0.1"
```

## What's here

- **Primitives**: `U256` (checked arithmetic), byte/address types.
- **`Error`**: the canonical contract error (e.g. `Error::Overflow`).
- **ABI**: the contract application binary interface types the macros generate
  against (selectors, encoded args/returns).
- **`IntentGoalType`**: the authoritative governance-goal enum every SDK binding
  locks against — `CONTRACT_DEPLOY`, `CONTRACT_CALL`, `CONTRACT_UPGRADE`,
  `SETTLEMENT`, `ROLE_ASSIGN`, `CAPABILITY_GRANT`, `DISCLOSURE_GRANT`,
  `CREDENTIAL_ISSUE`, `BRIDGE_SEND`/`BRIDGE_RECEIVE`, `SUBSYSTEM_ACTION`, … The
  same set the Go runtime (`pkg/intent`) and the JS SDK enforce, so a contract
  and the spine agree on the wire.
- **`DisclosureGrant` / `PrivacyClass`**: the selective-disclosure shapes
  (`Public … NeverDisclosable`, `ZkpOnly`).

Keeping these in one small crate is what lets contracts, the macros, and the
host bindings share one wire vocabulary without pulling heavy dependencies.
