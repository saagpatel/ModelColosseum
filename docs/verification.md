# Development and verification

Run commands from the repository root. The Rust manifest is
`src-tauri/Cargo.toml`; there is no root `Cargo.toml`.

## Prerequisites

- Node.js 22.22.0 or newer, as required by the locked React Router 8.4 package
  (a stricter minimum than Vite), and pnpm with support for the committed
  `pnpm-lock.yaml`.
- Rust stable with the `rustfmt` and `clippy` components. Native Tauri dependencies
  are required even for Rust tests: macOS desktop development needs Xcode Command
  Line Tools; see [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/).
  The [Rust CI workflow](../.github/workflows/test.yml) lists its Ubuntu libraries.
- Python 3 is optional for the existing synthetic contract checker below; it uses
  only the standard library.

Respect `pnpm-workspace.yaml` build permissions and dependency overrides. Use
frozen/locked installs; if a lockfile cannot be read, check the tool version
instead of regenerating it during verification. Ollama and installed models are
needed for interactive evaluation, not for these build and fixture checks.

## Checks without launching the app

The [canonical local gate](../.codex/verify.commands) performs a frozen pnpm
install, TypeScript/Vite production build, and the full locked Rust test suite.
Dependency installation may access package registries; use
`pnpm install --offline --frozen-lockfile` only when the packages are already cached.

For a small synthetic Run Boundary contract check:

```bash
python3 proof/evidence-boundary/verify_fixture.py
```

This reads only the committed schema and worked example. It does not launch the
app, query Ollama, open the live database, or prove Rust implementation behavior.

Use a focused Rust fixture lane for the module changed, for example:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib elo::tests::
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib run_boundary::tests::
```

The broader static checks are:

```bash
pnpm exec tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings
```

`pnpm test` is the Rust test alias, not a frontend test runner. The root Makefile
provides Rust-only `check`, `test`, `lint`, and release `build` targets using the
same manifest; `make build` does not build the frontend or a Tauri bundle.
There is no separate frontend lint or automated browser-test script configured.
[Rust CI](../.github/workflows/test.yml) runs Clippy and nextest with a cargo-test
fallback. [CodeQL](../.github/workflows/codeql.yml) analyzes JavaScript/TypeScript;
it does not replace the local frontend build.

## Interactive and visual verification

`pnpm tauri dev` and `make run` launch the desktop app. Startup opens
`~/.model-colosseum/colosseum.db` and may query local Ollama metadata. Use those
commands only when interactive app work is intended. `make clean` removes build
outputs and is not a verification gate.

Every mode of `script/build_and_run.sh` kills processes named `model-colosseum`.
Its `--verify` mode also launches a persistent development app; it is not an
offline smoke check. Do not use it as an unattended test or against an active
user session.

For changes to UI or exported reports, check the changed loading, empty, error,
success, and export paths in an app window or a browser with fixture/mock IPC,
and retain screenshots when visual behavior matters. Pure documentation changes
do not require browser checks. Run Boundary behavior has an existing
[integration and acceptance spec](evidence-boundary/run-boundary-integration-spec-v1.md).

For separately scoped desktop acceptance, the debug-only
`MODEL_COLOSSEUM_ACCEPTANCE_DB_PATH` override requires an absolute `.db` outside
the live data directory. The [acceptance fixture helper](../proof/evidence-boundary/create_acceptance_fixture.py)
requires an app-initialized empty disposable database and writes synthetic rows;
its `--verify-only` mode checks an already seeded fixture. Neither this override
nor the helper makes app startup offline. Preserve the live database and active
Ollama sessions when planning that lane.

Report which commands actually ran, failures or unavailable prerequisites, and
skipped interactive lanes. Local fixtures, builds, CI, and screenshots do not
establish live model quality, deployment reliability, cross-host transfer, or
human acceptance.
