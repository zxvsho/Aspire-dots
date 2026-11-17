#!/usr/bin/env bash
set -euo pipefail

# Convención de las apps aspire:
#   fuente   ~/.dots/aspire/<app>/
#   staging  ~/.dots/bin/<app>
#   enlace   ~/.local/bin/<app> -> ~/.dots/bin/<app>

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
DOTS_DIR="$(cd -- "$ROOT/../.." && pwd)"
STAGED_BIN="$DOTS_DIR/bin/aspire-files"
DEPLOYED_BIN="$HOME/.local/bin/aspire-files"

cargo build --release --manifest-path "$ROOT/Cargo.toml" -p aspire-files
install -Dm755 "$ROOT/target/release/aspire-files" "$STAGED_BIN"
mkdir -p "$(dirname -- "$DEPLOYED_BIN")"
ln -sfn "$STAGED_BIN" "$DEPLOYED_BIN"
printf 'Staging listo: %s\nEnlace creado: %s\n' "$STAGED_BIN" "$DEPLOYED_BIN"
