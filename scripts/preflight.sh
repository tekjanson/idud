#!/bin/bash
# Pre-flight checks before running idud-grow at scale

set -e

echo "🔍 idud Pre-Flight Validation"
echo "=============================="
echo ""

# Check 1: Binary exists
echo "✓ Checking binary..."
if [ ! -f "./target/release/idud" ]; then
    echo "❌ Binary not found. Run 'make build' first."
    exit 1
fi
echo "  ✓ Binary found"

# Check 2: Copilot CLI
echo ""
echo "✓ Checking Copilot CLI..."
if ! command -v copilot &> /dev/null; then
    echo "❌ Copilot CLI not found in PATH!"
    echo "   Install from: https://github.com/github/gh-copilot"
    exit 1
fi
COPILOT_VERSION=$(copilot --version 2>/dev/null || echo "unknown")
echo "  ✓ Copilot CLI available ($COPILOT_VERSION)"

# Check 3: Disk space
echo ""
echo "✓ Checking disk space..."
DISK_AVAILABLE=$(df -k . | tail -1 | awk '{print $4}')
DISK_MB=$((DISK_AVAILABLE / 1024))
if [ "$DISK_MB" -lt 1000 ]; then
    echo "⚠️  WARNING: Only ${DISK_MB}MB available. Training may fail."
else
    echo "  ✓ ${DISK_MB}MB available"
fi

# Check 4: Network connectivity (GitHub)
echo ""
echo "✓ Checking GitHub connectivity..."
if ! curl -s -m 5 https://api.github.com/users/github >/dev/null 2>&1; then
    echo "❌ Cannot reach GitHub. Check your internet connection."
    exit 1
fi
echo "  ✓ GitHub reachable"

# Check 5: Cache directory
echo ""
echo "✓ Checking cache directory..."
mkdir -p ./data/training_datalake
if [ -f "./data/training_datalake/training_cache.json" ]; then
    CACHE_ENTRIES=$(grep -c "repo_url" ./data/training_datalake/training_cache.json || echo 0)
    echo "  ✓ Cache exists with ~$CACHE_ENTRIES entries"
else
    echo "  ✓ Cache will be created on first run"
fi

echo ""
echo "✅ All pre-flight checks passed!"
echo ""
echo "Ready to run: make idud-grow [REPOS=N] [DURATION_MINUTES=M]"
echo ""
