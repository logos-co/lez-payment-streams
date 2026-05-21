# Logos operator basics (Nix builds, LGX, lgpm, logoscore)

This note summarizes how we run Logos Core locally while working on
`payment_streams_module`: which flakes produce which artifacts,
how `.lgx` packages are created,
how `lgpm` installs them,
and how `logoscore` loads them.

It complements the canonical CLI tables in
`../logos-tutorial/logos-developer-guide.md`
(check out `logos-tutorial` beside `lez-payment-streams` when paths matter)
and the pin-specific detail in
[`feature-branch-pins.md`](feature-branch-pins.md).

## Layout next to this repo

Keep a long-lived directory for installed core modules and tool binaries.
In our workspace it lives beside `lez-payment-streams`,
for example:

```text
lez-related/
  lez-payment-streams/          # this repository
  logos-cli/
    modules/                    # lgpm --modules-dir and logoscore -m
    package-manager/            # optional: nix out-link for lgpm
    logoscore-cli/              # optional: nix out-link for logoscore
```

`modules/` holds extracted packages (subdirectories with `manifest.json`).
Treat it as runtime state, not something to commit.

## Two Nix flakes inside `lez-payment-streams`

The repository root flake exposes `payment-streams-ffi` only.

The Logos Qt module lives under `logos-payment-streams-module/`,
which is a separate flake (it inputs `lez-payment-streams` via `path:..`).
You cannot add `#lgx` to the root flake by forwarding that subflake:
Nix reports a circular flake lock (`path:..` meets `path:./logos-payment-streams-module`).

Build commands:

```bash
# From lez-payment-streams/ — FFI crate only
nix build .#payment-streams-ffi

# From lez-payment-streams/ — Logos module bundle (preferred invocation)
nix build ./logos-payment-streams-module#lgx

# Equivalent after cd logos-payment-streams-module
nix build .#lgx
```

## Wallet module `.lgx` (PR 16 family)

Upstream PR 16 builds the wallet with plain CMake (not `mkLogosModule`),
so there is no `nix build .#lgx` on that flake alone.

Use the patched wrapper flake under
`logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched/`,
then bundle the `lib` output with `nix-bundle-lgx`:

```bash
cd logos-payment-streams-module/nix/flakes/logos-execution-zone-module-patched
nix bundle --bundler github:logos-co/nix-bundle-lgx .#lib -o ./wallet-lgx-out
```

Locate the generated `.lgx` under the bundler output directory (`find … -name '*.lgx'`).

The wrapper keeps `drv.src` so `nix-bundle-lgx` can read `metadata.json` at eval time
(it requires a `type` field).
See [`feature-branch-pins.md`](feature-branch-pins.md) for why the wrapper exists.

## Installing with lgpm

Install the dependency before dependents:

```bash
MODULES=/absolute/path/to/logos-cli/modules
lgpm --modules-dir "$MODULES" install --file /path/to/wallet.lgx
lgpm --modules-dir "$MODULES" install --file /path/to/payment_streams.lgx
lgpm --modules-dir "$MODULES" list
```

Use an absolute `$MODULES` path so commands do not depend on the current working directory.

After rebuilding a `.lgx`, run `lgpm install --file` again for that package
so the extracted tree matches the new store output.

Official command shapes also appear under “lgpm” in
`../logos-tutorial/logos-developer-guide.md`.

## Running logoscore

```bash
logoscore -D -m "$MODULES"
```

Use the same `$MODULES` directory `lgpm` populated.

Build `logoscore` and `lgpm` once via the developer guide flakes,
or enter an ephemeral environment:

```bash
nix shell github:logos-co/logos-package-manager#cli github:logos-co/logos-logoscore-cli
```

That avoids hand-editing global `PATH` with relative `./bin` segments.
For a reproducible shell with a lockfile,
a tiny flake under `logos-cli/` that exposes `devShells.default`
with those inputs is a reasonable next step.

## Step 6c runtime checks

Step 6c needs both `lez_wallet_module` and `payment_streams_module`
present under the same `modules/` tree before load-order and `lm` checks make sense.

Cross-reference:
[`integration-plan-v2.md`](../integration-plan-v2.md) Step 6c definition of done,
[`step6c-implementation-guidance.md`](step6c-implementation-guidance.md).
