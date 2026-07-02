# infrix-macros

Procedural macros that turn a plain Rust struct into an Infrix smart contract —
generating the ABI, dispatch, and governance wiring so you write business logic,
not boilerplate. Normally consumed transitively via [`infrix-sdk`](../infrix-sdk).

```toml
[dependencies]
infrix-macros = "0.1"
```

## Macros

| Macro | Purpose |
|-------|---------|
| `#[contract]` | Mark the struct as a contract; generates storage layout + ABI scaffolding. |
| `#[contract_impl]` | Mark the impl block whose methods become the contract's entry points. |
| `#[init]` | The constructor invoked on deploy. |
| `#[call]` | A state-mutating entry point (routed as a governed `CONTRACT_CALL`). |
| `#[view]` | A read-only entry point (no state change). |
| `#[event]` | Declare a typed event the contract can emit. |
| `#[storage_map]` | Declare a keyed storage map. |

### Governance macros

| Macro | Effect |
|-------|--------|
| `#[governed]` | Route the method through the governance spine. |
| `#[require_approval]` | Require an approval envelope before execution. |
| `#[require_capability]` | Gate on a capability grant. |
| `#[require_role]` | Gate on a role binding. |
| `#[evidenced]` | Emit structured evidence for the call. |

### Test macros

`#[infrix_test]` and `#[infrix_fuzz]` for contract unit + fuzz tests.

Each macro's generated code is deterministic and part of the contract's ABI —
see `infrix-sdk` for the end-to-end example.
