# 08 — aspire-music (reproductor de música en terminal)

Reproductor de música para la terminal estilo **LazyVim / cliamp**
(retro-moderno cyberpunk). TUI pura con Ratatui, motor de audio asíncrono
con Tokio y MPV. Este doc describe el esqueleto actual (v0.1.0).

---

## Qué es y estado actual

- Crate aparte en **Rust**: `~/.dots/aspire/aspire-music/`
  (`Cargo.toml`, `config.toml`, `src/main.rs` — todo en un fichero con
  9 secciones comentadas, pensado para ir troceándolo en módulos).
- **TUI**: Ratatui + Crossterm, render a ~60 FPS.
- **Runtime**: Tokio (`#[tokio::main]`). La UI y el motor corren en
  tareas independientes y se hablan por `tokio::sync::mpsc`
  (`AudioCommand` → motor, `AudioEvent` ← motor).
- **Motor de audio**: MPV como proceso hijo con socket IPC JSON
  (`--input-ipc-server`). Toca rutas locales y URLs de YouTube /
  SoundCloud vía el hook `yt-dlp` de MPV.
- **Spotify**: preparado, no activo:
  - stub `spotify_get_token()` (Client Credentials, solo si hay
    credenciales reales en config);
  - ruta híbrida activa: busco el nombre del track en YouTube con
    `yt-dlp` (`hybrid_search`) y lo reproduzco;
  - streaming nativo premium: dependencia `librespot` comentada en
    Cargo.toml + esbozo en la sección 6B del código.
- No comparte aspire-core (usa su propia paleta Tokyo Night /
  Catppuccin definida en código).

## Requisitos

```bash
pacman -S mpv yt-dlp    # imprescindibles
```

- **mpv**: motor. Sin él el menú abre pero sale toast de error.
- **yt-dlp**: resolver YouTube/SoundCloud (modo `ytsearch1:`).
- Cookies para YouTube/SoundCloud con restricciones (opcional):
  ruta en `config.toml` → `[auth] yt_cookies_path`
  (pasa a MPV como `--ytdl-raw-options=cookies=…`).

## Configuración

La app busca `config.toml`, en orden:

1. `./config.toml` — carpeta de trabajo (desarrollo: el del crate).
2. `~/.config/aspire-music/config.toml`.

```toml
[app]
lang = "es"                 # "es" | "en"
music_dir = "~/Music"       # escaneo de la fuente [1] Local

[theme]
name = "tokyo-night"        # "tokyo-night" | "catppuccin"

[auth]
spotify_client_id     = "TU_CLIENT_ID"
spotify_client_secret = "TU_CLIENT_SECRET"
yt_cookies_path = "~/.config/aspire-music/cookies.txt"

[player]
volume = 80                 # 0–130
```

Binario vivo: symlink `~/.local/bin/aspire-music` → snapshot estable en
`releases/aspire-music-<fecha>` (NO a `target/release`).

## Despliegue de versión (dos pasos: test dev + deploy)

El global **nunca** apunta a `target/release`: un `cargo build` roto no
puede romper el que ya funciona.

```bash
# DEV — aislado (cambios rotos o a medias no tocan lo global)
cd ~/.dots/aspire/aspire-music
cargo run --release                          # config: ./config.toml (cwd)
timeout 3 ./target/release/aspire-music      # 124 = abre bien

# DEPLOY — mandar la versión al PATH (paso deliberado, tras probar)
cp target/release/aspire-music releases/aspire-music-$(date +%Y%m%d)
ln -sf ~/.dots/aspire/aspire-music/releases/aspire-music-$(date +%Y%m%d) \
       ~/.local/bin/aspire-music

# ROLLBACK — instantáneo apuntando al snapshot anterior
ln -sf ~/.dots/aspire/aspire-music/releases/aspire-music-<fecha-previa> \
       ~/.local/bin/aspire-music
```

Notas:

- `cargo run` desde el crate usa el `config.toml` del crate (cwd), así que
  el dev testing queda aislado de la config global
  (`~/.config/aspire-music/config.toml`, todavía no creada).
- Snapshot actual: `releases/aspire-music-20261031` (esqueleto v0.1.0).

## Bind (sugerido, aún sin atar)

En `~/.config/mango/bind.conf`, interruptor como el resto:

```
bind=SUPER,m,spawn_shell,pkill -x aspire-music || ~/.local/bin/aspire-music
```

Tras editar: `mango -p` (0 errores) y `mmsg dispatch reload_config`.

## Arquitectura (por dentro)

```
┌────────────────────────────────────┐   mpsc    ┌─────────────────────────────────────┐
│ LOOP DE UI (tokio::main)           │ ─Commands─│ TAREA MOTOR (tokio::spawn)          │
│ Ratatui + Crossterm, ~60 FPS:      │           │ · lanza MPV una vez                 │
│  · poll teclado 8 ms               │           │ · consume AudioCommand              │
│  · dibuja cada 16 ms               │           │ · emite AudioEvent (posición cada   │
│  · try_recv de AudioEvent          │  ←Events──│   500 ms vía get_property time-pos) │
└────────────────────────────────────┘           └─────────────────────────────────────┘
```

- `AudioCommand`: `Play(Track) · TogglePause · SetVolume · Seek · Stop · Shutdown`.
- `AudioEvent`: `Position · Paused · VolumeChanged · Error`.
- Al salir (`q`) se envía `Shutdown`: el motor mata MPV.
- Socket MPV: `$XDG_RUNTIME_DIR/aspire-music-<pid>.sock`.

## Diseño (torcido a 60 FPS, fondo Tokyo Night)

```
┌──────────┬──────────────────────────────────────────┐
│ Sources  │  █████████  ███████╗...                  │  ← logo ASCII
│ ┃ [1] ●  │  (aspire / ◆ M U S I C ◆)                │
│   [2] ▶  ├──────────────────────────────────────────┤
│   [3] ♪  │ ▸ 01 track…   artista…  [--:--]          │
│   [4] ☁  │   02 track…                               │
│          │   03 track…   (/ = search)                │
├──────────┴──────────────────────────────────────────┤
│ barra de progreso (Gauge)                            │
│ PAUSE │ 🎵 Local │ VOL 80% │ j/k…  │   aspire-music │
└──────────────────────────────────────────────────────┘
```

- Paletas: `tokyo-night` (por defecto) y `catppuccin` (`[theme] name`).
- Barra inferior: verde `PLAYING`, amarillo `PAUSE`, rojo `STOPPED`.
- Panel lateral (24 cols) estilo Neo-tree con `┃` sobre la fuente activa.

## Atajos

| Tecla | Efecto |
|---|---|
| `1` `2` `3` `4` / `Tab` | Cambiar de fuente (Local/YT/Spotify/SoundCloud) |
| `j` `k` (`↑` `↓`) | Mover selección |
| `/` | Modo búsqueda (escribir y `Enter` para buscar en YT/Spotify/SoundCloud) |
| `Enter` | Reproducir lo seleccionado (o ejecutar la búsqueda) |
| `Espacio` | Pausa / reanudar |
| `n` / `p` | Siguiente / anterior de la fuente activa |
| `+` / `-` | Volumen ±5 |
| `←` / `→` | Seek ±10 s |
| `q` / `Ctrl+c` | Salir (mata MPV) |
| `Esc` | Cancelar búsqueda |

## Roadmap (TODOs marcados en el código)

- **librespot** (Spotify premium nativo): descomentar dependencia y
  montar `Session` en la sección 6B.
- Lector persistente de eventos MPV (hoy: pull best-effort 200 ms;
  MPV puede intercalar otros eventos).
- Duración de pista desde metadata → ratio real del Gauge.
- Separadores powerline reales en la barra inferior (glifo U+E0B0).
- Playlists / cola, búsqueda difusa, guardar favoritos.

## Problemas comunes

| Síntoma | Causa probable | Solución |
|---|---|---|
| Toast «No se pudo lanzar MPV» | MPV no instalado | `pacman -S mpv` |
| YouTube no reproduce | Falta yt-dlp / cookies | `pacman -S yt-dlp`; cookies en `[auth]` |
| Fuente Local vacía | `~/Music` no existe o no hay audio | crear directorio o ajustar `music_dir` |
| No abre / parpadea | Terminal sin raw mode / alt-screen | usar foot, kitty o alacritty (terminales del dotfile) |
| Cambia de tema | `theme.name` inválido | valores: `tokyo-night`, `catppuccin` |
