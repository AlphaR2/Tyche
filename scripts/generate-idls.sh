#!/bin/bash
set -e

# Ensure we're in the workspace root
cd "$(dirname "$0")/.."

# Generate IDL for tyche-core
echo "Generating IDL for tyche-core..."
shank idl --crate-root programs/tyche-core --out-dir clients/idls

# Generate IDL for tyche-escrow
echo "Generating IDL for tyche-escrow..."
shank idl --crate-root programs/tyche-escrow --out-dir clients/idls

# Generate IDL for tyche-auction
echo "Generating IDL for tyche-auction..."
shank idl --crate-root programs/tyche-auction --out-dir clients/idls

# Generate IDL for tyche-voter-weight-plugin
echo "Generating IDL for tyche-voter-weight-plugin..."
shank idl --crate-root programs/tyche-voter-weight-plugin --out-dir clients/idls

echo "All IDLs generated in clients/idls/"
