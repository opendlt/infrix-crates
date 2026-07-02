# counter — the canonical Infrix contract example

A minimal governed on-chain counter, built with [`infrix-sdk`](../../infrix-sdk).
It is the "hello world" for Infrix contract authoring and is verified to compile
to `wasm32-unknown-unknown`.

## Build

```sh
rustup target add wasm32-unknown-unknown   # once
cargo build --release --target wasm32-unknown-unknown
# → target/wasm32-unknown-unknown/release/counter.wasm
```

## Deploy (governed)

Contracts are never deployed by a raw route — deploy is a **governed intent**:

```sh
infrix intent submit --goal CONTRACT_DEPLOY \
  --user acc://alice.acme \
  --url  acc://alice.acme/counter \
  --wasm target/wasm32-unknown-unknown/release/counter.wasm
```

Then call it (also governed):

```sh
infrix intent submit --goal CONTRACT_CALL \
  --user acc://alice.acme --url acc://alice.acme/counter --function increment
```

## What it shows

- `#[contract]` / `#[contract_impl]` — struct → contract.
- `#[init]` — the deploy-time constructor.
- `#[call]` — a state-mutating, governed entry point (fail-loud on overflow).
- `#[view]` — a read-only entry point.

Copy this crate as the starting point for your own contract.
