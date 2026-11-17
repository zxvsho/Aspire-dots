#!/usr/bin/env bash
# ============================================================================
# Aspire Dots — compilador de apps
#
#   ./build.sh                 compila todas las apps y las deja en ~/.dots/bin
#   ./build.sh aspire-files    compila solo una app
#   ./build.sh --no-link       compila sin recrear los enlaces de ~/.local/bin
#   ./build.sh --dry-run       muestra lo que haría sin compilar
# ============================================================================

set -uo pipefail

DOTS_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib.sh
source "$DOTS_DIR/scripts/lib.sh"

usage() {
  printf 'Uso: %s [app|all] [--no-link] [--dry-run]\n' "$0"
  printf '\n'
  printf '  app        Nombre de la app (aspire-bar, aspire-bg, aspire-files, ...)\n'
  printf '  all        Compila todas (por defecto)\n'
  printf '  --no-link  No recrea los enlaces en ~/.local/bin\n'
  printf '  --dry-run  Muestra las acciones sin ejecutarlas\n'
  printf '\n'
  printf 'Apps disponibles:\n'
  local dir app
  for dir in "$DOTS_DIR"/aspire/*/; do
    [[ -f "$dir/Cargo.toml" ]] || continue
    app="$(basename "$dir")"
    aspire_is_lib_crate "$app" && continue
    printf '  - %s\n' "$app"
  done
}

TARGET="all"
LINK=1

while (($#)); do
  case "$1" in
    --no-link) LINK=0 ;;
    --dry-run) DRY_RUN=1 ;;
    -h|--help) usage; exit 0 ;;
    -*) usage >&2; exit 2 ;;
    *)
      [[ "$TARGET" == "all" ]] || { aspire_err "Solo puedes indicar una app."; exit 2; }
      TARGET="$1"
      ;;
  esac
  shift
done

if [[ "$TARGET" != "all" && ! -f "$DOTS_DIR/aspire/$TARGET/Cargo.toml" ]]; then
  aspire_die "No existe la app '$TARGET' en $DOTS_DIR/aspire"
fi

if ! aspire_build_apps "$DOTS_DIR/aspire" "$DOTS_DIR/bin" "$TARGET"; then
  aspire_err "Una o más apps fallaron al compilar."
  exit 1
fi

if [[ "$LINK" == 1 && "$DRY_RUN" != 1 ]]; then
  aspire_link_bins "$DOTS_DIR/bin"
fi

aspire_info "Binarios staged en $DOTS_DIR/bin"
