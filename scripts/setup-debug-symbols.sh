#!/bin/bash
# Aperture: Install debug symbols and configure kernel for symbol resolution.
# Run this inside the OrbStack VM (or any Linux profiling target).
#
# Usage: sudo bash scripts/setup-debug-symbols.sh
set -e

echo "=== Aperture Symbol Resolution Setup ==="

# 1. Enable kernel symbol exposure
echo ""
echo "--- Kernel Symbol Access ---"
KPTR=$(cat /proc/sys/kernel/kptr_restrict 2>/dev/null || echo "unknown")
echo "Current kptr_restrict: $KPTR"
if [ "$KPTR" != "0" ]; then
    sysctl -w kernel.kptr_restrict=0
    # Persist across reboots
    if [ -d /etc/sysctl.d ]; then
        echo "kernel.kptr_restrict = 0" > /etc/sysctl.d/99-aperture.conf
        echo "Persisted kptr_restrict=0 in /etc/sysctl.d/99-aperture.conf"
    fi
else
    echo "Already set to 0 (good)"
fi

# 2. Verify /proc/kallsyms has real addresses
echo ""
echo "--- /proc/kallsyms check ---"
FIRST=$(head -1 /proc/kallsyms 2>/dev/null || echo "unreadable")
if echo "$FIRST" | grep -q "^0000000000000000"; then
    echo "WARNING: kallsyms shows zeroed addresses even after kptr_restrict=0"
    echo "         Kernel symbols may not resolve. This can happen on some VM kernels."
else
    KSYM_COUNT=$(wc -l < /proc/kallsyms)
    echo "OK: $KSYM_COUNT kernel symbols available"
    echo "First entry: $FIRST"
fi

# 3. Install debug packages (Debian/Ubuntu)
echo ""
echo "--- Debug Packages ---"
if command -v apt-get &>/dev/null; then
    apt-get update -qq
    apt-get install -y --no-install-recommends libc6-dbg 2>/dev/null || \
        echo "Note: libc6-dbg not available (may already be installed)"
    echo "Installed libc6-dbg"
else
    echo "Not a Debian/Ubuntu system — install debug symbols manually"
fi

# 4. Summary
echo ""
echo "=== Summary ==="
echo "kptr_restrict: $(cat /proc/sys/kernel/kptr_restrict)"
echo "kallsyms:      $(wc -l < /proc/kallsyms) entries"
echo ""
echo "For Rust binaries, build with debug info:"
echo "  [profile.release]"
echo "  debug = true"
echo ""
echo "To verify a binary has debug symbols:"
echo "  file /path/to/binary    # should say 'not stripped'"
echo "  readelf -S /path/to/binary | grep debug"
echo ""
echo "Done."
