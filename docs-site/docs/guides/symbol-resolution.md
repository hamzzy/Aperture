---
sidebar_position: 2
title: Symbol Resolution
---

# Symbol Resolution Troubleshooting

When functions appear as raw hex addresses (e.g. `0xffff8b5b`) in the flamegraph or top functions table, it means the agent could not resolve the instruction pointer to a function name.

## Quick Fix

Run the setup script inside your profiling VM:

```bash
sudo bash scripts/setup-debug-symbols.sh
```

## Common Issues

### Kernel symbols show as hex

**Symptom:** All kernel frames (high addresses like `0xffff...`) are unresolved.

**Cause:** `kptr_restrict` is non-zero, preventing userspace from reading kernel symbol addresses.

**Fix:**

```bash
sudo sysctl kernel.kptr_restrict=0
```

**Verify:**

```bash
head -3 /proc/kallsyms
# Should show non-zero addresses, e.g.:
# ffff800080000000 T _text
```

### Userspace symbols show as hex

**Symptom:** Userspace function addresses are unresolved.

**Possible causes:**

1. **Binary is stripped** (no debug info):
   ```bash
   file /path/to/binary
   # "not stripped" = has symbols, "stripped" = no symbols
   ```

2. **Debug packages not installed** (for system libraries like glibc):
   ```bash
   sudo apt-get install libc6-dbg
   ```

3. **Process terminated** before symbol resolution. The agent resolves symbols by reading `/proc/PID/maps` during profiling. If the target process exits, its maps become unavailable.

4. **Wrong PID** in a namespace. In OrbStack or containers, ensure you pass the PID as seen from inside the namespace.

### Rust binaries

Ensure release builds include debug info:

```toml
# Cargo.toml
[profile.release]
debug = true        # full DWARF info
# or
debug = 1           # line tables only (smaller)
```

### OrbStack-specific notes

- The OrbStack kernel (`6.17.8-orbstack`) is a custom build. Some kernel functions may not appear in `/proc/kallsyms` even with `kptr_restrict=0`.
- **PID namespace:** The agent handles this automatically via `bpf_get_ns_current_pid_tgid()` for lock/syscall profiling. For CPU profiling, pass the namespace-local PID with `--pid`.

## Verifying Symbol Resolution

Run the agent with `--verbose` to see detailed symbol resolution logs:

```bash
sudo aperture-agent --pid 1234 --duration 10s --verbose
```

Look for lines like:

```
Symbol resolution: 45/60 user IPs, 12/15 kernel IPs resolved (cache: 92 entries)
```

- If kernel resolution shows `0/N`, check `kptr_restrict`.
- If user resolution shows `0/N`, check that the target process is running and has debug info.

## How Symbol Resolution Works

The agent uses [blazesym](https://github.com/libbpf/blazesym) to resolve instruction pointers to function names. Two code paths exist:

### Local Path

Used when generating local output (flamegraph SVG, JSON). Splits IPs by address range:

- `>= 0xffff_0000_0000_0000` — kernel symbols (via `/proc/kallsyms`)
- All others — userspace symbols (via `/proc/PID/maps` + debug info)

### Aggregator Push Path

Pre-resolves symbols before sending to the aggregator. Same kernel/user split. The resolved symbol string includes module info:

```
function_name [module_basename]
```

For example: `__schedule [vmlinux]` or `malloc [libc.so.6]`.
