#!/usr/bin/env bash
set -euo pipefail

DOTS_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$DOTS_DIR/aspire/aspire-bar"
STAGED_BIN="$DOTS_DIR/bin/aspire-bar"
DEPLOYED_BIN="$HOME/.local/bin/aspire-bar"

usage() {
  printf 'Uso: %s [build|deploy]\n' "$0"
  printf '  build   Compila aspire-bar y lo copia a ~/.dots/bin/aspire-bar\n'
  printf '  deploy  Copia el binario de staging a ~/.local/bin/aspire-bar\n'
}

command="${1:-build}"
if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

case "$command" in
  build)
    cargo build --release --manifest-path "$CRATE_DIR/Cargo.toml"
    install -Dm755 "$CRATE_DIR/target/release/aspire-bar" "$STAGED_BIN"
    printf 'Staging listo: %s\n' "$STAGED_BIN"
    ;;
  deploy)
    if [[ ! -x "$STAGED_BIN" ]]; then
      printf 'No existe un binario staged ejecutable: %s\n' "$STAGED_BIN" >&2
      printf 'Ejecuta primero: %s build\n' "$0" >&2
      exit 1
    fi
    install -Dm755 "$STAGED_BIN" "$DEPLOYED_BIN"
    printf 'Desplegado: %s\n' "$DEPLOYED_BIN"
    ;;
  help|-h|--help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
