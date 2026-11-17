#!/usr/bin/env bash
# ============================================================================
# Aspire Dots — instalador para Arch Linux y CachyOS
#
#   ./install.sh                 instalación completa
#   ./install.sh --skip-packages no instala paquetes (solo configs y build)
#   ./install.sh --skip-backup   no crea copia de seguridad
#   ./install.sh --skip-restore  no copia config/ ni dots/ a ~
#   ./install.sh --skip-build    no compila las apps Aspire
#   ./install.sh --skip-path     no toca ~/.local/bin ni el PATH
#   ./install.sh --dry-run       muestra lo que haría sin ejecutar
# ============================================================================

set -uo pipefail

REPO_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=dots/scripts/lib.sh
source "$REPO_DIR/dots/scripts/lib.sh"

SKIP_PACKAGES=0
SKIP_BACKUP=0
SKIP_RESTORE=0
SKIP_BUILD=0
SKIP_PATH=0
ASPIRE_BACKUP_DIR=""

usage() {
  printf 'Uso: %s [opciones]\n\n' "$0"
  printf '  --skip-packages  No instala paquetes de pacman ni de AUR\n'
  printf '  --skip-backup    No crea copia de seguridad en ~/.aspire-backup\n'
  printf '  --skip-restore   No copia config/ ni dots/ a ~\n'
  printf '  --skip-build     No compila las apps Aspire\n'
  printf '  --skip-path      No enlaza binarios ni modifica el PATH\n'
  printf '  --dry-run        Muestra las acciones sin ejecutarlas\n'
  printf '  -h, --help       Muestra esta ayuda\n'
}

while (($#)); do
  case "$1" in
    --skip-packages) SKIP_PACKAGES=1 ;;
    --skip-backup)   SKIP_BACKUP=1 ;;
    --skip-restore)  SKIP_RESTORE=1 ;;
    --skip-build)    SKIP_BUILD=1 ;;
    --skip-path)     SKIP_PATH=1 ;;
    --dry-run)       DRY_RUN=1 ;;
    -h|--help)       usage; exit 0 ;;
    *) aspire_err "Opción desconocida: $1"; usage >&2; exit 2 ;;
  esac
  shift
done

aspire_require_arch

aspire_step "Aspire Dots — instalación"
aspire_info "Repo: $REPO_DIR"
aspire_info "Destino: $HOME"

# ---------------------------------------------------------------------------
# 1. Copia de seguridad
# ---------------------------------------------------------------------------
if [[ "$SKIP_BACKUP" == 0 ]]; then
  aspire_backup
else
  aspire_info "Copia de seguridad omitida"
fi

# ---------------------------------------------------------------------------
# 2. Paquetes
# ---------------------------------------------------------------------------
if [[ "$SKIP_PACKAGES" == 0 ]]; then
  aspire_sync_repo
  aspire_install_repo_packages "$REPO_DIR/packages/arch.txt" || \
    aspire_warn "Algunos paquetes oficiales no se instalaron."

  if helper="$(aspire_ensure_aur_helper)"; then
    [[ -n "$helper" ]] || aspire_die "No se pudo preparar yay ni paru."
    aspire_install_aur_packages "$REPO_DIR/packages/aur.txt" "$helper" || \
      aspire_warn "Algunos paquetes AUR no se instalaron."
  else
    aspire_warn "No se pudo preparar un helper de AUR; se omiten los paquetes AUR."
  fi
else
  aspire_info "Instalación de paquetes omitida"
fi

# ---------------------------------------------------------------------------
# 3. Configuraciones
# ---------------------------------------------------------------------------
if [[ "$SKIP_RESTORE" == 0 ]]; then
  aspire_restore_configs "$REPO_DIR"
else
  aspire_info "Restauración de configuraciones omitida"
fi

# ---------------------------------------------------------------------------
# 4. Compilar apps
# ---------------------------------------------------------------------------
if [[ "$SKIP_BUILD" == 0 ]]; then
  aspire_build_apps "$REPO_DIR/dots/aspire" "$HOME/.dots/bin" || \
    aspire_warn "Algunas apps no se compilaron correctamente."
else
  aspire_info "Compilación omitida"
fi

# ---------------------------------------------------------------------------
# 5. Enlaces y PATH
# ---------------------------------------------------------------------------
if [[ "$SKIP_PATH" == 0 ]]; then
  aspire_link_bins "$HOME/.dots/bin"
  aspire_ensure_path
else
  aspire_info "Enlaces y PATH omitidos"
fi

# ---------------------------------------------------------------------------
# 6. Resumen
# ---------------------------------------------------------------------------
aspire_lazyvim_note
aspire_summary "$ASPIRE_BACKUP_DIR"
