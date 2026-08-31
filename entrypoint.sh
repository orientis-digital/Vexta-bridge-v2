#!/bin/sh
set -e

# Ensure SQLite data directory exists and has correct ownership for vexta user
mkdir -p /app/data
chown -R vexta:vexta /app/data /app/admin-ui 2>/dev/null || true
chmod 700 /app/data 2>/dev/null || true

# Execute the bridge binary as non-root vexta user
exec su-exec vexta /app/vexta-bridge-v2 "$@"
