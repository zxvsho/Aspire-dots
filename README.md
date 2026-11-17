# Aspire Dots

Dotfiles para **Arch Linux / CachyOS** con **MangoWM** en Wayland: configuración
del escritorio, temas, scripts y las apps propias **Aspire** escritas en Rust +
GTK4.

El repositorio contiene **solo configuración y código fuente**. Los binarios se
compilan en la máquina destino con `install.sh` o `dots/build.sh`.

## Requisitos

- Arch Linux o CachyOS (repositorios oficiales `core` / `extra`).
- `pacman` y conexión a internet.
- `base-devel`, `git`, `rust` y `pkgconf` (el instalador los añade).
- GTK4 y `gtk4-layer-shell` para las apps Aspire (también los añade).

El instalador funciona igual en Arch y en CachyOS porque usa los mismos
nombres de paquete de los repositorios oficiales. Para AUR detecta `paru` o
`yay`; si no existe ninguno, clona y construye `paru` automáticamente.

## Instalación

```bash
git clone https://github.com/zxvsho/Aspire-dots.git
cd Aspire-dots
./install.sh
```

El instalador:

1. Crea una copia de seguridad en `~/.aspire-backup/<fecha>/`.
2. Actualiza el sistema e instala `packages/arch.txt`.
3. Detecta o construye un helper de AUR e instala `packages/aur.txt`.
4. Copia `config/` a `~/.config/`, `dots/` a `~/.dots/` y los archivos del
   shell (`.zshrc`, `.bashrc`, `.bash_profile`, `.gitconfig`) a `~`.
5. Compila todas las apps Aspire y las deja en `~/.dots/bin/`.
6. Crea los symlinks en `~/.local/bin/` y asegura el `PATH`.

Al terminar, cierra sesión y vuelve a entrar.

### Opciones

```bash
./install.sh --skip-packages   # solo configuración y compilación
./install.sh --skip-backup     # no crea copia de seguridad
./install.sh --skip-restore    # no copia config/ ni dots/ a ~
./install.sh --skip-build      # no compila las apps
./install.sh --skip-path       # no enlaza binarios ni toca el PATH
./install.sh --dry-run         # muestra las acciones sin ejecutarlas
```

## Estructura

```text
Aspire-dots/
├── install.sh              # Instalador Arch/CachyOS
├── packages/
│   ├── arch.txt            # Paquetes oficiales (pacman)
│   └── aur.txt             # Paquetes AUR
├── config/                 # Configuraciones -> ~/.config/
├── dots/                   # Lógica y recursos -> ~/.dots/
│   ├── aspire/             # Código fuente de las apps Aspire
│   ├── assets/             # Wallpapers, iconos, temas de Waybar
│   ├── bin/                # Staging (vacío en el repo; se genera al compilar)
│   ├── Manual/             # Documentación
│   ├── scripts/            # Scripts legacy + lib.sh
│   └── build.sh            # Compilador de apps
├── .zshrc
├── .bashrc
├── .bash_profile
└── .gitconfig
```

## Apps Aspire

| App | Qué hace | Config |
|---|---|---|
| `aspire-bar` | Editor visual de Waybar para MangoWM | `~/.config/aspire-bar/` |
| `aspire-bg` | Selector de fondos y motor de color (matugen/tinty). Backend por defecto: `swaybg` | `~/.config/aspire-bg/` |
| `aspire-clip` | Historial de portapapeles con vista previa (lee `cliphist`) | `~/.config/aspire-clip/` |
| `aspire-files` | Gestor de archivos GTK4 (workspace de 4 crates) | `~/.config/aspire-files/config.toml` |
| `aspire-menu` | Menú global del dotfile | `~/.config/rofi-menu/menu.toml` |
| `aspire-music` | Reproductor de música TUI (Ratatui + MPV) | config del crate |
| `aspire-themes` | Selector de temas con vista previa | `~/.config/aspire-themes/` |
| `aspire-tui` | Experimentos TUI | — |
| `aspire-core` | Librería compartida (motor de color y paleta). No genera binario | — |

`aspire-files` vive en `dots/aspire/aspire-files/` y agrupa los crates
`aspire-files`, `aspire-fs`, `aspire-config` y `aspire-thumb`.

## Compilar a mano

```bash
~/.dots/build.sh                 # todas las apps
~/.dots/build.sh aspire-files    # solo una app
~/.dots/build.sh --no-link       # sin recrear ~/.local/bin
~/.dots/build.sh --dry-run       # ver qué haría
```

Los binarios quedan en `~/.dots/bin/<app>` y `~/.local/bin/<app>` apunta a
ellos. `aspire-core` se compila como dependencia, pero no se instala como
ejecutable.

## Paquetes

Las listas están separadas para que sea fácil añadir o quitar cosas:

- `packages/arch.txt`: repositorios oficiales (`pacman`). Funciona en Arch y
  CachyOS.
- `packages/aur.txt`: AUR (se instala con `paru` o `yay`).

Entre otros se instalan `mangowm`, `waybar`, `matugen`, `tinty-git`, `nwg-look`,
`swaybg`, `rofi`, `fuzzel`, `mako`, `foot`, `rio`, `neovim`, `yazi`, `imv`,
`mpv`, `obs-studio`, `zen-browser-bin` y las fuentes JetBrains Mono Nerd,
Victor Mono Nerd, Noto y `ttf-ms-fonts`.

`swaybg` es el backend de fondos por defecto de `aspire-bg`. No se necesita
`awww` ni `swww`.

### Apps de solo configuración

Estas apps **no se instalan**, pero su configuración se conserva en `config/`
para quien las tenga o las quiera usar después:

`nautilus`, `nemo`, `libfm`, `thunar`, `kitty`, `alacritty`, `ghostty`,
`xfce4-terminal`, `zed`, `visual-studio-code-bin`, `micro`, `chromium`,
`google-chrome`, `onlyoffice`, `qalculate`, `zim`, `drawio`, `mangobar`.

## LazyVim

`config/nvim/` es la base de LazyVim. El repositorio **no incluye** los plugins
ni `lazy.nvim`. La primera vez que abras `nvim` con conexión a internet,
`lua/config/lazy.lua` clona `lazy.nvim` y LazyVim instala todo automáticamente.
No hace falta ejecutar nada más.

## Fondos y tema

- `aspire-bg` usa `swaybg` por defecto y guarda su estado en
  `~/.config/aspire-bg/config.ini`.
- La paleta se genera con `matugen` o `tinty` y se publica en
  `~/.config/theme-engine/colors.css`.
- Las apps Aspire leen esa paleta y se actualizan en caliente.

## Secretos y datos que no se versionan

El `.gitignore` excluye credenciales, sesiones, caches y estado de ejecución:

- `config/gh/hosts.yml`
- `config/opencode/service.json`, `config/opencode/auth.json`
- `config/pulse/`
- `config/Aurelia/`
- `config/aspire-bg/wallhaven/`
- `config/zen/`, `config/Code/`, `config/chromium/`, `config/google-chrome/`
- `config/dconf/`, historiales, backups `*.bak-*`, `config/theme-engine/.rendered-*`

Si clonaste una versión anterior del repositorio, **rota** las credenciales que
hayas tenido en `config/Aurelia/session.json`, `config/opencode/service.json` y
`config/pulse/cookie`.

## Copias de seguridad

Cada ejecución de `install.sh` guarda `~/.config`, `~/.dots` y los archivos del
shell en:

```text
~/.aspire-backup/YYYYMMDD-HHMMSS/
```

Puedes restaurar manualmente cualquier archivo desde ahí.
