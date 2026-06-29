# Payment streams module — setup

Prerequisites for Flow A and for Store integration (Flow B builds the same wallet + module
artifacts, plus `delivery_module`).

## Tooling shell

Example (adjust paths):

```bash
nix shell \
  github:logos-co/logos-package-manager \
  github:logos-co/logos-logoscore-cli \
  github:logos-co/logos-module#lm \
  --command bash
```

## Scaffold and localnet

From repo root: `lgs init`, `lgs setup`, `lgs localnet start` (see
[step10a-local-chain-fixture.md](../step10a-local-chain-fixture.md) in the development map for
seed script detail).

`make seed-fixture` runs [scripts/seed-localnet-fixture.sh](../../scripts/seed-localnet-fixture.sh).

Flow A module prepare does not require the Store funded snapshot; it only needs localnet running.

## Module and wallet artifacts

Flow A build (no delivery):

```bash
MODE=module CHAIN=local ./scripts/e2e.sh build
```

Or build the payment streams `.lgx` directly:

```bash
nix build ./logos-payment-streams-module#lgx
```

Wallet: patched `logos_execution_zone` (see [feature-branch-pins.md](../feature-branch-pins.md)).
Flow B / unified build may invoke the wallet `.lgx` builder via `e2e.sh`.

## Guest ELF

Set on the logoscore process:

```bash
export PAYMENT_STREAMS_GUEST_BIN="$REPO/methods/guest/target/riscv32im-risc0-zkvm-elf/docker/lez_payment_streams.bin"
```

Rebuild after guest changes:

```bash
cargo risczero build --manifest-path methods/guest/Cargo.toml
```

## Pins and patches

[feature-branch-pins.md](../feature-branch-pins.md) — LEZ rc5 operational pin, delivery fork,
wallet wrapper flakes.

Historical step runbooks (10b, 11a–11d): [development-map/README.md](../development-map/README.md).

Extended install loop: [logos-runtime-guide.md](../logos-runtime-guide.md) (development map).
