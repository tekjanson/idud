# idud-hygiene

`idud-hygiene` is a manifest-driven architecture and code-quality engine. It can be used as a CLI, as a JSON-driven automation tool, or as a reusable contract layer for AI-assisted development.

## Why this is AI-friendly

- The core contract is JSON, so an AI can generate or edit manifests without understanding Rust.
- The CLI supports `--json` so any language can consume the output over stdout.
- The project can ship native binaries through the release workflow, so you do not need the Rust toolchain to run it.

## Quick start

Build locally:

```bash
cargo build --release -p idud-hygiene
```

Run against the repository:

```bash
./target/release/idud-hygiene --report --json . crates/idud-hygiene/golden_patterns
```

## Binary release workflow

A GitHub Actions workflow at `.github/workflows/release-idud-hygiene.yml` builds native binaries for Linux, macOS, and Windows and uploads them as workflow artifacts.
