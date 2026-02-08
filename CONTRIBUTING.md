# Contributing to Aperture

Thank you for your interest in contributing to Aperture. This document covers the development setup, coding standards, and pull request process.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Layout](#project-layout)
- [Building](#building)
- [Testing](#testing)
- [Coding Standards](#coding-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)
- [License](#license)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be respectful, constructive, and collaborative.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:

   ```bash
   git clone https://github.com/YOUR_USERNAME/aperture.git
   cd aperture
   ```

3. **Add upstream** remote:

   ```bash
   git remote add upstream https://github.com/yourusername/aperture.git
   ```

4. **Create a branch** for your work:

   ```bash
   git checkout -b feature/my-feature main
   ```

## Development Setup

### Prerequisites

| Tool | Version | Required for |
| ---- | ------- | ------------ |
| Rust stable | latest | Agent, aggregator, CLI, shared crates |
| Rust nightly | latest | eBPF programs (`agent-ebpf/`) |
| `rust-src` component | (nightly) | eBPF build with `-Zbuild-std=core` |
| Node.js | 18+ | Web dashboard (`ui/`) |
| Docker + Docker Compose | latest | ClickHouse, full-stack testing |
| Linux kernel | 5.10+ | Running the agent (eBPF) |
| `bpf-linker` | latest | Linking eBPF programs |

### Install Rust toolchains

```bash
# Stable (workspace default)
rustup install stable
rustup component add rustfmt clippy

# Nightly (eBPF only)
rustup install nightly
rustup component add rust-src --toolchain nightly
```

### Install bpf-linker (for eBPF development)

```bash
cargo install bpf-linker
```

### Install Node.js dependencies (for UI development)

```bash
cd ui && npm install
```

## Repository Structure

```text
aperture/
├── agent/                 # Userspace profiling agent (loads eBPF, resolves symbols)
├── agent-ebpf/            # eBPF programs (no_std, bpfel-unknown-none target)
│   └── src/
│       ├── cpu_profiler.rs    # perf_event CPU sampling
│       ├── lock_profiler.rs   # futex tracepoint tracing
│       └── syscall_tracer.rs  # raw tracepoint syscall tracking
├── shared/                # Shared types, wire protocol (bincode + base64), utilities
├── aggregator/            # Aggregation service
│   ├── src/
│   │   ├── server/        # gRPC + HTTP servers
│   │   ├── alerts.rs      # Alert engine (rules, evaluation, history)
│   │   ├── aggregate.rs   # Batch aggregation logic
│   │   ├── buffer.rs      # In-memory ring buffer
│   │   ├── export.rs      # JSON + collapsed-stack export
│   │   ├── storage/       # ClickHouse persistence
│   │   └── metrics.rs     # Prometheus metrics
│   └── proto/             # gRPC protobuf definitions
├── cli/                   # CLI client (query, aggregate, diff)
├── wasm-runtime/          # WASM filter runtime (wasmtime 16)
├── gpu-profiler/          # GPU profiling (CUDA/CUPTI, work in progress)
├── ui/                    # React web dashboard (Vite + Tailwind + shadcn/ui)
├── docs-site/             # Docusaurus documentation site
├── deploy/k8s/            # Kubernetes manifests (Namespace, DaemonSet, Deployment)
├── scripts/               # Setup and install scripts
├── .github/workflows/     # CI/CD (GitHub Actions)
├── docker-compose.yml     # Full-stack Docker setup
├── Dockerfile.agent       # Multi-stage agent build
└── Dockerfile.aggregator  # Multi-stage aggregator build
```


### Crate dependency graph

```
agent-ebpf (no_std, kernel)
    ↓ perf events
agent (userspace)
    ↓ uses
shared (types, wire protocol)
    ↑ uses          ↑ uses
aggregator        cli
    ↑ uses
wasm-runtime
```

## Building

### Workspace (aggregator, CLI, shared, agent)

```bash
# Build everything except eBPF
cargo build --workspace

# Build specific binary
cargo build --release --bin aperture-aggregator
cargo build --release --bin aperture-cli
```

### eBPF programs

eBPF programs require nightly and must target `bpfel-unknown-none`:

```bash
cargo +nightly build -Zbuild-std=core --target bpfel-unknown-none \
  --bin cpu-profiler --bin lock-profiler --bin syscall-tracer
```

Or use the cargo alias:

```bash
cargo +nightly build-ebpf
```

### Web dashboard

```bash
cd ui
npm install
npm run build       # Production build
npm run dev         # Development server at http://localhost:5173
```

### Documentation site

```bash
cd docs-site
npm install
npm start           # Development server at http://localhost:3000
npm run build       # Production build
```

## Testing

### Rust tests

```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p aperture-agent
cargo test -p aperture-aggregator
cargo test -p aperture-shared
cargo test -p aperture-cli
cargo test -p aperture-wasm

# Run a specific test
cargo test -p aperture-aggregator -- alerts::tests::test_evaluate_fires
```

### UI tests

```bash
cd ui
npm run test          # Run once
npm run test:watch    # Watch mode
```

### Integration tests

The aggregator has an end-to-end test that requires ClickHouse:

```bash
docker compose up -d clickhouse
export APERTURE_CLICKHOUSE_ENDPOINT="http://127.0.0.1:8123"
export APERTURE_CLICKHOUSE_DATABASE="aperture"
export APERTURE_CLICKHOUSE_PASSWORD="e2etest"
cargo test -p aperture-aggregator -- --include-ignored
```

### Linting

```bash
# Rust
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# UI
cd ui && npm run lint

# Documentation site
cd docs-site && npm run build   # Catches broken links
```

## Coding Standards

### Rust

- **Format**: Run `cargo fmt` before every commit. CI will reject unformatted code.
- **Clippy**: Fix all `cargo clippy` warnings. Use `#[allow(...)]` sparingly and with a comment explaining why.
- **Edition**: 2021 (set in workspace `Cargo.toml`).
- **Error handling**: Use `anyhow::Result` for application code, `thiserror` for library error types. Avoid `.unwrap()` in non-test code.
- **Unsafe**: Minimize `unsafe`. When required (eBPF, WASM memory), add a `// SAFETY:` comment explaining the invariant.
- **Dependencies**: Add new dependencies to `[workspace.dependencies]` in the root `Cargo.toml` and reference them with `.workspace = true` in crate-level `Cargo.toml` files.
- **Tests**: Add tests for new functionality. Place unit tests in `#[cfg(test)] mod tests` within the source file. Use `tests/` directory for integration tests.

### TypeScript / React (UI)

- **TypeScript**: All UI code is TypeScript. No `any` types without justification.
- **Components**: Use functional components with hooks. Follow existing shadcn/ui patterns.
- **API hooks**: Use `@tanstack/react-query` for all server state. Add hooks in `ui/src/api/queries.ts`.
- **Styling**: Tailwind CSS utility classes. Follow existing patterns in the codebase.

### eBPF

- **no_std**: eBPF programs cannot use the standard library. Use `#![no_std]` and `#![no_main]`.
- **Verifier safety**: All code paths must terminate. Avoid unbounded loops. The kernel verifier will reject programs that might not terminate.
- **Map access**: Always check return values from map lookups. The verifier requires null checks.

### Commits

- Write clear, descriptive commit messages.
- Use imperative mood: "Add lock profiler" not "Added lock profiler".
- Reference issues where applicable: "Fix buffer overflow in ring buffer (#42)".
- Keep commits focused — one logical change per commit.

## Pull Request Process

### Before opening a PR

1. **Sync with upstream**:

   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run the full check suite**:

   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cd ui && npm run lint && npm run test && npm run build
   ```

3. **Check that the docs site builds** (if you changed docs):

   ```bash
   cd docs-site && npm run build
   ```

### PR requirements

- **Title**: Short, descriptive (under 70 characters). Prefix with area: `agent:`, `aggregator:`, `ui:`, `docs:`, `ebpf:`, `ci:`.
- **Description**: Explain what changed and why. Include "Fixes #issue" if applicable.
- **Tests**: Add or update tests for any behavioral changes.
- **No unrelated changes**: Keep PRs focused. Refactoring and feature work should be separate PRs.
- **CI must pass**: All GitHub Actions checks must be green before merge.

### PR title examples

```
agent: add verbose symbol resolution logging
aggregator: fix borrow checker error in alert evaluation
ui: add syscall latency histogram component
ebpf: handle PID namespace filtering for lock profiler
docs: add WASM filter development guide
ci: add aarch64 cross-compilation to release workflow
```

### Review process

1. A maintainer will review your PR within a reasonable timeframe.
2. Address review feedback by pushing new commits (don't force-push during review).
3. Once approved, a maintainer will merge the PR.

## Issue Guidelines

### Bug reports

Include:

- Aperture version or commit hash
- Operating system and kernel version (`uname -r`)
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs (run with `--verbose` for agent issues)

### Feature requests

Include:

- Clear description of the desired behavior
- Use case — why is this needed?
- Any prior art or references

### Good first issues

Issues labeled `good-first-issue` are suitable for new contributors. They are typically well-scoped and have context about the expected approach.

## License

By contributing to Aperture, you agree that your contributions will be dual-licensed under the [Apache License 2.0](LICENSE-APACHE) and [MIT License](LICENSE-MIT), consistent with the project's existing license terms.
