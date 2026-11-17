#!/usr/bin/env bash
# ============================================================================
# Aspire Dots — helpers compartidos por install.sh y dots/build.sh
# No ejecuta nada por sí solo: solo define funciones.
# ============================================================================

# ---------------------------------------------------------------------------
# Salida
# ---------------------------------------------------------------------------
if [[ -t 1 ]]; then
  C_G='\033[0;32m'; C_Y='\033[1;33m'; C_R='\033[0;31m'; C_B='\033[1;34m'; C_N='\033[0m'
else
  C_G=''; C_Y=''; C_R=''; C_B=''; C_N=''
fi

aspire_step() { printf '\n%b==> %s%b\n' "$C_B" "$*" "$C_N" >&2; }
aspire_info() { printf '%b  ·%b %s\n' "$C_G" "$C_N" "$*" >&2; }
aspire_warn() { printf '%b  ! %s%b\n' "$C_Y" "$*" "$C_N" >&2; }
aspire_err()  { printf '%b  x %s%b\n' "$C_R" "$*" "$C_N" >&2; }
aspire_die()  { aspire_err "$*"; exit 1; }
aspire_have() { command -v "$1" >/dev/null 2>&1; }

# Registro de fallos recuperables: el instalador sigue y los resume al final.
ASPIRE_FAILURES=()
aspire_fail() {
  ASPIRE_FAILURES+=("$*")
  aspire_warn "$*"
}

DRY_RUN="${DRY_RUN:-0}"

aspire_run() {
  if [[ "$DRY_RUN" == 1 ]]; then
    printf '%b  [dry-run]%b %s\n' "$C_Y" "$C_N" "$*"
  else
    "$@"
  fi
}

# ---------------------------------------------------------------------------
# Comprobaciones del sistema
# ---------------------------------------------------------------------------
aspire_require_arch() {
  [[ $EUID -ne 0 ]] || aspire_die "No ejecutes este script como root."
  [[ -f /etc/arch-release ]] || aspire_die "Este instalador es para Arch Linux / CachyOS (no se encontró /etc/arch-release)."
  aspire_have pacman || aspire_die "No se encontró pacman."
}

# ---------------------------------------------------------------------------
# Listas de paquetes
# ---------------------------------------------------------------------------
aspire_read_list() {
  local file=$1
  [[ -f "$file" ]] || { aspire_warn "No existe la lista de paquetes: $file"; return 0; }
  sed -e 's/#.*//' -e 's/[[:space:]]//g' "$file" | grep -v '^$'
}

# ---------------------------------------------------------------------------
# Paquetes oficiales (Arch / CachyOS)
# ---------------------------------------------------------------------------
aspire_sync_repo() {
  aspire_step "Actualizando índices y sistema (pacman -Syu)"
  if aspire_run sudo pacman -Syu --noconfirm; then
    return 0
  fi

  aspire_fail "pacman -Syu falló; intentando refrescar el llavero y continuar."
  aspire_run sudo pacman -Sy --noconfirm archlinux-keyring 2>/dev/null || true
  aspire_run sudo pacman -Syu --noconfirm || \
    aspire_fail "pacman -Syu sigue fallando; se intentarán los paquetes uno a uno."
  return 0
}

aspire_install_repo_packages() {
  local file=$1
  local -a pkgs=()
  mapfile -t pkgs < <(aspire_read_list "$file")
  ((${#pkgs[@]})) || return 0

  aspire_step "Paquetes oficiales (${#pkgs[@]})"
  if aspire_run sudo pacman -S --needed --noconfirm "${pkgs[@]}"; then
    return 0
  fi

  aspire_warn "La instalación conjunta falló; reintentando uno a uno..."
  local p
  local -a failed=()
  for p in "${pkgs[@]}"; do
    aspire_run sudo pacman -S --needed --noconfirm "$p" || failed+=("$p")
  done
  if ((${#failed[@]})); then
    aspire_fail "No se pudieron instalar paquetes oficiales: ${failed[*]}"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# AUR: detectar yay/paru o construir paru
# ---------------------------------------------------------------------------
aspire_detect_aur_helper() {
  local h
  for h in paru yay; do
    if aspire_have "$h"; then
      printf '%s\n' "$h"
      return 0
    fi
  done
  return 1
}

aspire_build_paru() {
  aspire_step "No hay yay ni paru: construyendo paru desde AUR"
  if [[ "$DRY_RUN" == 1 ]]; then
    aspire_info "[dry-run] clonaría y construiría paru"
    printf 'paru\n'
    return 0
  fi
  aspire_have makepkg || { aspire_fail "makepkg no está disponible (instala base-devel); se omiten los paquetes AUR."; return 1; }
  aspire_have git || { aspire_fail "git no está disponible; se omiten los paquetes AUR."; return 1; }

  local tmp
  tmp="$(mktemp -d)"

  # paru-bin evita compilar; si falla, se compila paru desde fuente.
  if aspire_run git clone --depth 1 https://aur.archlinux.org/paru-bin.git "$tmp/paru-bin"; then
    if (cd "$tmp/paru-bin" && aspire_run makepkg -si --noconfirm); then
      rm -rf "$tmp"
      printf 'paru\n'
      return 0
    fi
  fi

  aspire_warn "paru-bin falló; intentando compilar paru desde fuente..."
  rm -rf "$tmp/paru-bin"
  if aspire_run git clone --depth 1 https://aur.archlinux.org/paru.git "$tmp/paru"; then
    if (cd "$tmp/paru" && aspire_run makepkg -si --noconfirm); then
      rm -rf "$tmp"
      printf 'paru\n'
      return 0
    fi
  fi

  rm -rf "$tmp"
  return 1
}

aspire_ensure_aur_helper() {
  local helper
  if helper="$(aspire_detect_aur_helper)"; then
    aspire_info "AUR helper detectado: $helper"
    printf '%s\n' "$helper"
    return 0
  fi
  aspire_build_paru
}

aspire_install_aur_packages() {
  local file=$1 helper=$2
  local -a pkgs=()
  mapfile -t pkgs < <(aspire_read_list "$file")
  ((${#pkgs[@]})) || return 0

  aspire_step "Paquetes AUR (${#pkgs[@]}) con $helper"
  if aspire_run "$helper" -S --needed --noconfirm "${pkgs[@]}"; then
    return 0
  fi

  aspire_warn "La instalación conjunta AUR falló; reintentando uno a uno..."
  local p
  local -a failed=()
  for p in "${pkgs[@]}"; do
    aspire_run "$helper" -S --needed --noconfirm "$p" || failed+=("$p")
  done
  if ((${#failed[@]})); then
    aspire_fail "No se pudieron instalar paquetes AUR: ${failed[*]}"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# Verificación de paquetes críticos
# ---------------------------------------------------------------------------
aspire_verify_packages() {
  local p
  local -a missing=()
  if [[ "$DRY_RUN" == 1 ]]; then
    aspire_info "[dry-run] verificaría paquetes críticos"
    return 0
  fi
  local -a critical=(
    mangowm waybar matugen swaybg neovim rofi fuzzel mako foot rio yazi imv
    gtk4 gtk4-layer-shell gdk-pixbuf2 pkgconf rust poppler ffmpegthumbnailer
    cliphist tinty-git wlogout xfce-polkit ttf-ms-fonts oh-my-posh-bin
  )
  for p in "${critical[@]}"; do
    pacman -Q "$p" >/dev/null 2>&1 || missing+=("$p")
  done
  if ((${#missing[@]})); then
    aspire_fail "Faltan paquetes críticos: ${missing[*]}"
    return 1
  fi
  aspire_info "Paquetes críticos verificados."
}

# ---------------------------------------------------------------------------
# Copia de seguridad
# ---------------------------------------------------------------------------
aspire_backup() {
  local dest
  dest="$HOME/.aspire-backup/$(date +%Y%m%d-%H%M%S)"
  ASPIRE_BACKUP_DIR="$dest"
  aspire_step "Copia de seguridad en $dest"

  if [[ "$DRY_RUN" == 1 ]]; then
    aspire_run mkdir -p "$dest"
    return 0
  fi

  if ! mkdir -p "$dest"; then
    ASPIRE_BACKUP_DIR=""
    aspire_fail "No se pudo crear $dest; se continúa sin copia de seguridad."
    return 1
  fi
  local item
  for item in .config .dots .zshrc .bashrc .bash_profile .gitconfig; do
    [[ -e "$HOME/$item" ]] || continue
    cp -a "$HOME/$item" "$dest/" || aspire_fail "No se pudo copiar ~/$item al respaldo."
  done
  aspire_info "Respaldo listo: $dest"
}

# ---------------------------------------------------------------------------
# Restaurar configuraciones del repo
# ---------------------------------------------------------------------------
aspire_restore_configs() {
  local repo=$1
  local rc=0
  aspire_step "Restaurando configuraciones"

  aspire_run mkdir -p "$HOME/.config" "$HOME/.dots" || {
    aspire_fail "No se pudieron crear ~/.config o ~/.dots."
    return 1
  }

  if [[ -d "$repo/config" ]]; then
    if aspire_run cp -a "$repo/config/." "$HOME/.config/"; then
      aspire_info "config/  ->  ~/.config"
    else
      aspire_fail "No se pudo copiar config/ a ~/.config."
      rc=1
    fi
  fi

  if [[ -d "$repo/dots" ]]; then
    if aspire_run cp -a "$repo/dots/." "$HOME/.dots/"; then
      aspire_info "dots/    ->  ~/.dots"
    else
      aspire_fail "No se pudo copiar dots/ a ~/.dots."
      rc=1
    fi
  fi

  local f
  for f in .zshrc .bashrc .bash_profile .gitconfig; do
    [[ -f "$repo/$f" ]] || continue
    if aspire_run cp -a "$repo/$f" "$HOME/$f"; then
      aspire_info "$f  ->  ~/"
    else
      aspire_fail "No se pudo copiar $f a ~/."
      rc=1
    fi
  done
  return "$rc"
}

# ---------------------------------------------------------------------------
# Compilar apps Aspire
# ---------------------------------------------------------------------------
ASPIRE_LIB_CRATES=(core)

aspire_is_lib_crate() {
  local app=$1 c
  for c in "${ASPIRE_LIB_CRATES[@]}"; do
    [[ "$app" == "$c" ]] && return 0
  done
  return 1
}

aspire_build_apps() {
  local apps_dir=$1 stage_dir=$2 filter=${3:-all}
  aspire_step "Compilando apps Aspire"

  if ! aspire_have cargo; then
    aspire_fail "cargo no está instalado (paquete rust); se omiten las apps Aspire."
    return 1
  fi
  if [[ ! -d "$apps_dir" ]]; then
    aspire_fail "No existe el directorio de apps: $apps_dir"
    return 1
  fi

  local dir app bin
  local -a built=() failed=()

  for dir in "$apps_dir"/*/; do
    [[ -d "$dir" ]] || continue
    app="$(basename "$dir")"
    [[ -f "$dir/Cargo.toml" ]] || continue
    if [[ "$filter" != "all" && "$app" != "$filter" ]]; then
      continue
    fi

    if aspire_is_lib_crate "$app"; then
      aspire_info "Saltando $app (librería compartida)"
      continue
    fi

    if [[ "$DRY_RUN" == 1 ]]; then
      aspire_info "[dry-run] compilaría $app"
      built+=("$app")
      continue
    fi

    aspire_info "Compilando $app..."
    if ! cargo build --release --manifest-path "$dir/Cargo.toml"; then
      aspire_warn "Falló la compilación de $app; reintentando una vez..."
      if ! cargo build --release --manifest-path "$dir/Cargo.toml"; then
        aspire_fail "Falló la compilación de $app."
        failed+=("$app")
        continue
      fi
    fi

    bin="$dir/target/release/$app"
    if [[ -x "$bin" ]]; then
      if install -Dm755 "$bin" "$stage_dir/$app"; then
        built+=("$app")
        aspire_info "Staging: $stage_dir/$app"
      else
        aspire_fail "No se pudo copiar $app a $stage_dir."
        failed+=("$app")
      fi
    else
      aspire_fail "No se generó el binario esperado: $bin"
      failed+=("$app")
    fi
  done

  if ((${#failed[@]})); then
    aspire_warn "Apps con problemas: ${failed[*]}"
  fi
  if ((${#built[@]})); then
    aspire_info "Apps compiladas: ${built[*]}"
  fi
  ((${#failed[@]} == 0))
}

# ---------------------------------------------------------------------------
# Enlaces y PATH
# ---------------------------------------------------------------------------
aspire_link_bins() {
  local stage_dir=$1
  local rc=0
  aspire_step "Enlazando binarios en ~/.local/bin"
  if ! aspire_run mkdir -p "$HOME/.local/bin"; then
    aspire_fail "No se pudo crear ~/.local/bin."
    return 1
  fi

  local bin app
  for bin in "$stage_dir"/*; do
    [[ -f "$bin" && -x "$bin" ]] || continue
    app="$(basename "$bin")"
    if aspire_run ln -sfn "$bin" "$HOME/.local/bin/$app"; then
      aspire_info "$app"
    else
      aspire_fail "No se pudo enlazar $app en ~/.local/bin."
      rc=1
    fi
  done
  return "$rc"
}

aspire_ensure_path() {
  aspire_step "Asegurando PATH (~/.local/bin y ~/.dots/bin)"
  local marker='# >>> aspire path >>>'
  local endmark='# <<< aspire path <<<'
  local rc=0
  local rcfile

  for rcfile in "$HOME/.zshrc" "$HOME/.bashrc"; do
    if [[ "$DRY_RUN" == 1 ]]; then
      aspire_run bash -c "grep -qF '$marker' '$rcfile' || printf '%s\\n' '$marker' >> '$rcfile'"
      continue
    fi
    [[ -f "$rcfile" ]] || : >"$rcfile"
    if ! grep -qF "$marker" "$rcfile"; then
      if {
        printf '\n%s\n' "$marker"
        printf 'case ":$PATH:" in *":$HOME/.local/bin:"*) ;; *) export PATH="$HOME/.local/bin:$PATH" ;; esac\n'
        printf 'case ":$PATH:" in *":$HOME/.dots/bin:"*) ;; *) export PATH="$HOME/.dots/bin:$PATH" ;; esac\n'
        printf '%s\n' "$endmark"
      } >>"$rcfile"; then
        aspire_info "PATH añadido a $rcfile"
      else
        aspire_fail "No se pudo actualizar $rcfile con el PATH."
        rc=1
      fi
    else
      aspire_info "PATH ya presente en $rcfile"
    fi
  done
  return "$rc"
}

# ---------------------------------------------------------------------------
# LazyVim
# ---------------------------------------------------------------------------
aspire_lazyvim_note() {
  aspire_step "Neovim / LazyVim"
  aspire_info "LazyVim se instala solo en el primer arranque de nvim."
  aspire_info "Abre nvim una vez con conexión a internet y espera a que termine."
}

# ---------------------------------------------------------------------------
# Resumen
# ---------------------------------------------------------------------------
aspire_summary() {
  local backup=${1:-}
  aspire_step "Listo"
  if [[ -n "$backup" ]]; then
    aspire_info "Respaldo: $backup"
  fi
  aspire_info "Cierra sesión y vuelve a entrar para recargar PATH y servicios."
  aspire_info "Fondo de pantalla por defecto: swaybg (gestionado por aspire-bg)."
  aspire_info "Primer arranque de nvim: LazyVim se descarga automáticamente."

  if ((${#ASPIRE_FAILURES[@]})); then
    aspire_step "Avisos de la instalación"
    local f
    for f in "${ASPIRE_FAILURES[@]}"; do
      aspire_warn "$f"
    done
    aspire_info "La instalación terminó, pero revisa los avisos anteriores."
    return 1
  fi

  aspire_info "Instalación completada sin errores."
  return 0
}
