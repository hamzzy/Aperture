#!/bin/bash
set -e

REMOTE_HOST="ubuntu@orb"
REMOTE_DIR="~/aperture"

echo "Syncing to $REMOTE_HOST..."
# Ensure remote directory exists
ssh $REMOTE_HOST "mkdir -p $REMOTE_DIR"

# Sync files (excluding heavy build artifacts)
rsync -avz \
    --exclude 'target' \
    --exclude '.git' \
    --exclude '.claude' \
    --exclude '.DS_Store' \
    ./ $REMOTE_HOST:$REMOTE_DIR

echo "Running tests on $REMOTE_HOST..."
ssh $REMOTE_HOST "cd $REMOTE_DIR && cargo test --workspace -- --nocapture"
