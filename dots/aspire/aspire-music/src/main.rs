//! ╔══════════════════════════════════════════════════════════════════════════╗
//! ║  aspire-music — Reproductor de música para la terminal                   ║
//! ╚══════════════════════════════════════════════════════════════════════════╝
//!
//! ─────────────────────────────────────────────────────────────────────────────
//! ARQUITECTURA GENERAL (ver Manual 02 — estructura-general)
//!
//!   ┌──────────────────────────────────────┐    tokio::sync::mpsc    ┌────────────────────────────────────┐
//!   │  LOOP DE UI (tokio::main)            │  ── AudioCommand ──▶    │  TAREA MOTOR DE AUDIO (tokio::spawn)│
//!   │  · Ratatui + Crossterm               │                         │  · Backend MPV (IPC JSON socket)    │
//!   │  · Dibuja a ~60 FPS (frame fijo)     │  ◀── AudioEvent ────    │  · yt-dlp hook (YouTube/SoundCloud) │
//!   │  · Traduce teclado → comandos        │                         │  · [stub] librespot (Spotify)       │
//!   └──────────────────────────────────────┘                         └────────────────────────────────────┘
//!
//!  • El hilo UI NUNCA toca el motor: solo encola comandos y consume eventos.
//!  • El motor vive en una tarea Tokio independiente → puede hacer IO (spawn
//!    de procesos, sockets) sin congelar el render.
//!  • Aspecto visual: estilo "LazyVim / cliamp" (retro-moderno cyberpunk):
//!    paleta Tokyo Night / Catppuccin, bordes redondeados Unicode, lateral
//!    izquierdo tipo Neo-tree y barra inferior tipo Lualine.
//!
//!  SECCIONES de este fichero (mentalmente modular, todo en main.rs para arrancar):
//!    1. Config   (config.toml)
//!    2. I18n     (es / en)
//!    3. Paleta   (Tokyo Night / Catppuccin)
//!    4. Modelo   (Track / Source)
//!    5. Canales  (AudioCommand / AudioEvent)
//!    6. Motor    (MPV IPC + Spotify híbrido + stub librespot)
//!    7. Estado   (App)
//!    8. Render   (logo, sidebar, lista, progreso, Lualine)
//!    9. Bucle principal
//! ─────────────────────────────────────────────────────────────────────────────

// ═══════════════════════════════════════════════════════════════════════════════
// IMPORTS
// ═══════════════════════════════════════════════════════════════════════════════

use std::{
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Direction, Rect},
    prelude::*,
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{
        block::Block,
        Gauge, List, ListItem, ListState, Paragraph,
    },
    Terminal,
};
use serde::Deserialize;
use tokio::sync::mpsc;

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 1 — CONFIGURACIÓN CENTRALIZADA (config.toml)
// ═══════════════════════════════════════════════════════════════════════════════

/// NoD desfavorecido
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app:    AppSection,
    pub theme:  ThemeSection,
    pub auth:   AuthSection,
    pub player: PlayerSection,
}
/// NoD desfavorecido
#[derive(Debug, Clone, Deserialize)]
pub struct AppSection {
    pub lang: String,
    #[serde(default = "default_music_dir")]
    pub music_dir: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeSection { pub name: String }
#[derive(Debug, Clone, Deserialize)]
pub struct AuthSection {
    #[serde(default)] pub spotify_client_id: String,
    #[serde(default)] pub spotify_client_secret: String,
    #[serde(default)] pub yt_cookies_path: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSection {
    #[serde(default = "default_volume")] pub volume: u8,
}
fn default_music_dir() -> String { "~/Music".to_string() }
fn default_volume() -> u8 { 80 }

impl AppConfig {
    /// Carga config.toml. Orden de búsqueda:
    ///   1. ./config.toml          (carpeta del proyecto — desarrollo)
    ///   2. ~/.config/aspire-music/config.toml
    /// Si no existe ninguna, aplica valores por defecto.
    pub fn cargar() -> Self {
        let paths = [
            PathBuf::from("config.toml"),
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("aspire-music/config.toml"),
        ];
        for p in paths {
            if let Ok(raw) = std::fs::read_to_string(&p) {
                match toml::from_str::<AppConfig>(&raw) {
                    Ok(cfg) => return cfg,
                    Err(e) => eprintln!("⚠ config.toml inválido ({p:?}): {e}"),
                }
            }
        }
        // Config por defecto (degradación elegante)
        toml::from_str(&String::new()).expect("imposible: config vacía")
    }
}

/// Impl defaults: necesarios porque toml::from_str de un string vacío requiere defaults.
impl Default for AppConfig { fn default() -> Self { toml::from_str(concat!(
    "[app]\n", "lang = \"es\"\n",
    "[theme]\n", "name = \"tokyo-night\"\n",
    "[auth]\n",
    "[player]\n", "volume = 80\n",
)).unwrap() } }

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 2 — INTERNACIONALIZACIÓN (I18N: es / en)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang { #[default] Es, En }

impl Lang {
    pub fn parse(s: &str) -> Self {
        match s { "en" => Lang::En, _ => Lang::Es }
    }
    /// Traduce una clave de UI al idioma actual.
    /// Claves: sources, local, youtube, spotify, soundcloud,
    ///         search, normal, playing, paused, stopped, volume,
    ///         hint, quit, path, playing_now
    pub fn t(self, key: &str) -> &'static str {
        match (self, key) {
            (Lang::Es, "sources")    => "  Fuentes",
            (Lang::Es, "local")      => "Local",
            (Lang::Es, "youtube")    => "YouTube",
            (Lang::Es, "spotify")    => "Spotify",
            (Lang::Es, "soundcloud") => "SoundCloud",
            (Lang::Es, "search")     => "Buscar",
            (Lang::Es, "normal")     => "Normal",
            (Lang::Es, "playing")    => "Reproduc.",
            (Lang::Es, "paused")     => "Pausa",
            (Lang::Es, "stopped")    => "Deten.",
            (Lang::Es, "volume")     => "VOL",
            (Lang::Es, "hint")       => " j/k mover · Enter reproducir · / buscar · 1-4 fuente · +/- vol · q salir",
            (Lang::Es, "path")       => "~/.config/aspire-music",
            (Lang::Es, "playing_now")=> "Reproduciendo",
            (Lang::En, "sources")    => "  Sources",
            (Lang::En, "local")      => "Local",
            (Lang::En, "youtube")    => "YouTube",
            (Lang::En, "spotify")    => "Spotify",
            (Lang::En, "soundcloud") => "SoundCloud",
            (Lang::En, "search")     => "Search",
            (Lang::En, "normal")     => "Normal",
            (Lang::En, "playing")    => "Playing",
            (Lang::En, "paused")     => "Pause",
            (Lang::En, "stopped")    => "Stopped",
            (Lang::En, "volume")     => "VOL",
            (Lang::En, "hint")       => " j/k move · Enter play · / search · 1-4 source · +/- vol · q quit",
            (Lang::En, "path")       => "~/.config/aspire-music",
            (Lang::En, "playing_now")=> "Now playing",
            _ => "",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 3 — PALETA (Tokyo Night / Catppuccin)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg:     Color,
    pub panel:  Color,
    pub fg:     Color,
    pub faint:  Color,
    pub purple: Color,
    pub blue:   Color,
    pub cyan:   Color,
    pub green:  Color,
    pub red:    Color,
    pub yellow: Color,
    pub border: Color,
}

impl Palette {
    pub fn by_name(name: &str) -> Self {
        match name {
            "catppuccin" => Palette::catppuccin(),
            _            => Palette::tokyo_night(),
        }
    }

    /// Paleta Tokyo Night — fondo azul/noche profundo, texto lavanda, acentos verde reativas.
    pub fn tokyo_night() -> Self {
        Palette {
            bg:     Color::Rgb(0x1a, 0x1b, 0x26),
            panel:  Color::Rgb(0x16, 0x16, 0x1e),
            fg:     Color::Rgb(0xc0, 0xca, 0xf5),
            faint:  Color::Rgb(0x56, 0x5f, 0x89),
            purple: Color::Rgb(0xbb, 0x9a, 0xf7),
            blue:   Color::Rgb(0x7a, 0xa2, 0xf7),
            cyan:   Color::Rgb(0x7d, 0xcf, 0xff),
            green:  Color::Rgb(0x9e, 0xce, 0x6a),
            red:    Color::Rgb(0xf7, 0x76, 0x8e),
            yellow: Color::Rgb(0xe0, 0xaf, 0x68),
            border: Color::Rgb(0x3b, 0x42, 0x61),
        }
    }
    /// Paleta Catppuccin Mocha — alternativa cálida tipo "latte nocturno".
    pub fn catppuccin() -> Self {
        Palette {
            bg:     Color::Rgb(0x1e, 0x1e, 0x2e),
            panel:  Color::Rgb(0x18, 0x18, 0x25),
            fg:     Color::Rgb(0xc9, 0xd3, 0xf8),
            faint:  Color::Rgb(0x6c, 0x70, 0x86),
            purple: Color::Rgb(0xcb, 0xa6, 0xf7),
            blue:   Color::Rgb(0x89, 0xb4, 0xfa),
            cyan:   Color::Rgb(0x94, 0xe2, 0xd5),
            green:  Color::Rgb(0xa6, 0xe3, 0xa1),
            red:    Color::Rgb(0xf3, 0x89, 0x9b),
            yellow: Color::Rgb(0xf9, 0xe2, 0xaf),
            border: Color::Rgb(0x45, 0x47, 0x5a),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 4 — MODELO DE DATOS (Track / Source)
// ═══════════════════════════════════════════════════════════════════════════════

/// Fuentes de música disponibles (atajos 1..4 en la TUI, tipo Neo-tree).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source { Local, YouTube, Spotify, SoundCloud }

impl Source {
    /// Todas las fuentes en orden — coincide con los dígitos 1..4.
    pub const ALL: [Source; 4] = [Source::Local, Source::YouTube, Source::Spotify, Source::SoundCloud];
    pub fn index(self) -> usize { Source::ALL.iter().position(|s| *s == self).unwrap() }
    pub fn digit(self) -> &'static str { ["1","2","3","4"][self.index()] }
    pub fn icon(self) -> &'static str { ["", "▶", "", "☁"][self.index()] }
    pub fn key(self) -> &'static str { ["local","youtube","spotify","soundcloud"][self.index()] }
}

/// Una pista, homogénea para todas las fuentes.
#[derive(Debug, Clone)]
pub struct Track {
    pub id:       String,          // id interno (aplicaciones futuras: spotify_id, video_id…)
    pub title:    String,
    pub artist:   String,
    pub uri:      String,          // ruta local, URL de YT/SC, o "ytdlp://" resuelto
    pub duration: Option<u64>,     // segundos (si se conoce)
    pub source:   Source,
}

impl Track {
    fn new(source: Source, title: impl Into<String>, uri: impl Into<String>) -> Self {
        Track { id: String::new(), title: title.into(), artist: String::new(), uri: uri.into(), duration: None, source }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 5 — CANALES ASÍNCRONOS (tokio::sync::mpsc)
// ═══════════════════════════════════════════════════════════════════════════════

/// Comandos que la TUI envía al motor.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Empieza a reproducir una pista (URL de YT/SC, ruta local, uri spotify…).
    Play(Track),
    /// Alterna pausa/reproducción.
    TogglePause,
    /// Volumen 0..=130 (MPV acepta hasta 130 con gain).
    SetVolume(u8),
    /// Salto relativo en segundos (±10 habitualmente).
    Seek(i64),
    /// Detiene la reproducción actual.
    Stop,
    /// Apaga el motor (mata MPV y finaliza la tarea).
    Shutdown,
}

/// Eventos que el motor envía a la TUI.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Posición actual de reproducción en segundos.
    Position(f64),
    /// Estado de pausa actual.
    Paused(bool),
    /// El volumen cambió (confirmación del motor).
    VolumeChanged(u8),
    /// Error recuperable (no mata la app).
    Error(String),
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 6 — MOTOR DE AUDIO (Backend MPV via IPC JSON)
// ═══════════════════════════════════════════════════════════════════════════════
//
//  MPV acepta URLs de YouTube/SoundCloud directamente porque usa el hook
//  `ytdl-hook`, que llama a `yt-dlp` para resolver el stream. Si configuras
//  `yt_cookies_path` en config.toml, se le pasa `--ytdl-raw-options=cookies=…`.
//
//  Protocolo IPC: socket Unix + una línea JSON por comando
//    →  {"command":["loadfile","<url>","play"]}\n
//  Respuesta también en JSON; MPV puede emitir muchos eventos, por eso la
//  lectura es "best effort" (skeleton — TODO: bucle de lectura persistente).

/// El backend MPV: proceso hijo + socket de control.
struct MpvBackend {
    child:       tokio::process::Child,
    socket_path: PathBuf,
}

impl MpvBackend {
    /// Lanza mpv en modo daemon "idle" con un socket IPC dedicado.
    async fn launch(cookies: Option<PathBuf>) -> Result<Self> {
        let socket_path = dirs::runtime_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(format!("aspire-music-{}.sock", std::process::id()));
        // Limpia restos de ejecuciones previas — evita "connect refused".
        let _ = std::fs::remove_file(&socket_path);

        let mut args = vec![
            "--no-video".to_string(),
            "--idle=yes".to_string(),          // se queda vivo sin reproducir nada
            "--keep-open=no".to_string(),
            "--no-terminal".to_string(),
            "--ytdl-format=bestaudio/best".to_string(), // prioriza audio para música
            format!("--input-ipc-server={}", socket_path.display()),
        ];
        if let Some(c) = cookies {
            args.push(format!("--ytdl-raw-options=cookies={}", c.display()));
        }

        let child = tokio::process::Command::new("mpv")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("¿Tienes MPV instalado? (pacman -S mpv)")?;

        // Pequeña espera para que el socket exista antes del primer comando.
        tokio::time::sleep(Duration::from_millis(120)).await;
        Ok(Self { child, socket_path })
    }

    /// Envía un comando JSON a MPV (conexión efímera por comando — skeleton).
    async fn send(&self, command: serde_json::Value) {
        use tokio::io::AsyncWriteExt;
        if let Ok(mut s) = tokio::net::UnixStream::connect(&self.socket_path).await {
            let mut msg = command.to_string().into_bytes();
            msg.push(b'\n');
            let _ = s.write_all(&msg).await;
        }
    }
    async fn loadfile(&self, uri: &str) { self.send(serde_json::json!({"command":["loadfile", uri, "play"]})).await }
    async fn toggle_pause(&self) { self.send(serde_json::json!({"command":["cycle","pause"]})).await }
    async fn set_volume(&self, vol: i64) { self.send(serde_json::json!({"command":["set","volume", vol]})).await }
    async fn seek(&self, sec: i64) { self.send(serde_json::json!({"command":["seek", sec, "relative"]})).await }
    async fn stop(&self) { self.send(serde_json::json!({"command":["stop"]})).await }

    fn kill(mut self) { let _ = self.child.start_kill(); }
}

/// Consulta "time-pos" a MPV (posición actual en segundos).
/// Skeleton: abre conexión efímera, escribe la pregunta y lee UNA línea.
async fn mpv_now(socket: &Path) -> Option<f64> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let stream = tokio::net::UnixStream::connect(socket).await.ok()?;
    let mut rd = tokio::io::BufReader::new(stream);
    let cmd = serde_json::json!({"command":["get_property","time-pos"]});
    rd.write_all(format!("{cmd}\n").as_bytes()).await.ok()?;
    let mut line = String::new();
    // MPV puede mandar eventos antes de esta respuesta; con timeout corto,
    // si atrapamos otro mensaje lo ignoramos (mejora futura: lector dedicado).
    tokio::time::timeout(Duration::from_millis(200), rd.read_line(&mut line))
        .await.ok()?.ok()?;
    let val: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    val.get("data")?.as_f64()
}

/// ── 6B · STUB DE SPOTIFY ──────────────────────────────────────────────────────
//
//  · streaming premium nativo (librespot): descomenta la dependencia en
//    Cargo.toml y aquí inicializarías una `Session` librespot con device_id.
//    Ejemplo esbozado:
//
//      use librespot::core::session::Session;
//      use librespot::core::authentication::Credentials;
//      let credentials = Credentials::with_password(user, pass);
//      let session = Session::new(config, None);
//      session.connect(credentials, true).await?;
//
//  · sistema híbrido: busca el nombre del track en YouTube y devuelve una URL
//    reproducible por MPV (ruta que YA funciona con este esqueleto).

/// Búsqueda híbrida: usa yt-dlp para resolver `query` a una URL reproducible.
/// Funciona para Spotify (título de canción) y SoundCloud (search).
async fn hybrid_search(query: &str) -> Option<String> {
    let mut cmd = tokio::process::Command::new("yt-dlp");
    cmd.args(["--get-id", "--no-warnings", &format!("ytsearch1:{query}")]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    let out = cmd.output().await.ok()?;
    let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if id.is_empty() { None } else { Some(format!("https://youtube.com/watch?v={id}")) }
}

/// Obtiene un token de acceso de la Web API de Spotify (Client Credentials).
/// TODO: usarlo para listar playlists / tracks en la fuente [3] Spotify.
async fn spotify_get_token(client_id: &str, client_secret: &str) -> Option<String> {
    if client_id.is_empty() || client_id.contains("TU_") { return None; }
    let client = reqwest::Client::new();
    let body = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];
    let resp = client
        .post("https://accounts.spotify.com/api/token")
        .form(&body)
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("access_token")?.as_str().map(|s| s.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 6C — TAREA DEL MOTOR DE AUDIO (tokio::spawn)
// ═══════════════════════════════════════════════════════════════════════════════

/// Tarea dedicada que vive todo el ciclo de la app:
///   · lanza MPV una vez,
///   · consume AudioCommand (del backend del UI),
///   · emite AudioEvent (posición, pausa, errores, volumen).
async fn engine_task(
    mut cmd_rx: mpsc::UnboundedReceiver<AudioCommand>,
    ev_tx: mpsc::UnboundedSender<AudioEvent>,
    cookies: Option<PathBuf>,
) {
    let mut mpv: Option<MpvBackend> = match MpvBackend::launch(cookies).await {
        Ok(m)  => Some(m),
        Err(e) => {
            let _ = ev_tx.send(AudioEvent::Error(format!("No se pudo lanzar MPV: {e}")));
            None
        }
    };
    let socket    = mpv.as_ref().map(|m| m.socket_path.clone());
    let mut tick  = tokio::time::interval(Duration::from_millis(500));
    let mut paused = false;
    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else { break };  // canal cerrado → salir
                match (mpv.as_ref(), cmd) {
                    (Some(m), AudioCommand::Play(t))      => { m.loadfile(&t.uri).await; paused = false; let _ = ev_tx.send(AudioEvent::Paused(false)); }
                    (Some(m), AudioCommand::TogglePause)  => { m.toggle_pause().await;   paused = !paused;      let _ = ev_tx.send(AudioEvent::Paused(paused)); }
                    (Some(m), AudioCommand::SetVolume(v)) => { m.set_volume(v as i64).await; let _ = ev_tx.send(AudioEvent::VolumeChanged(v)); }
                    (Some(m), AudioCommand::Seek(s))      => { m.seek(s).await; }
                    (Some(m), AudioCommand::Stop)         => { m.stop().await;   paused = false; }
                    (Some(_), AudioCommand::Shutdown)     => break,
                    (None, _)                             => { let _ = ev_tx.send(AudioEvent::Error("MPV no está disponible".into())); }
                }
            }
            _ = tick.tick() => {
                // Emitir posición periódicamente a la UI (solo si está reproduciendo).
                if let (Some(s), false) = (socket.as_ref(), paused) {
                    if let Some(p) = mpv_now(s).await {
                        let _ = ev_tx.send(AudioEvent::Position(p));
                    }
                }
            }
        }
    }
    if let Some(m) = mpv.take() { m.kill(); }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 7 — ESTADO DE LA TUI (App)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode { Normal, Search }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayState { Playing, Paused, Stopped }

/// Estado central de la interfaz.
struct App {
    // Config
    lang:    Lang,
    pal:     Palette,
    #[allow(dead_code)] // TODO: hot-reload de config.toml y rutas del render
    cfg:     AppConfig,
    // Navegación
    source:  Source,               // fuente activa (1..4)
    sel_idx: ListState,            // selección de pista en la lista
    mode:    Mode,
    search:  String,               // buffer de búsqueda
    // Datos
    tracks:  Vec<Vec<Track>>,      // una lista por Source::ALL
    // Reproductor
    paused:  bool,
    play_state: PlayState,
    volume:  u8,
    position: f64,                 // segundos
    // Canales (para comandos desde callbacks / clicks futuros)
    cmd_tx: mpsc::UnboundedSender<AudioCommand>,
    // Mensaje efímero en pantalla (errores / avisos)
    toast:  Option<(String, std::time::Instant)>,
}

impl App {
    fn new(cfg: AppConfig, cmd_tx: mpsc::UnboundedSender<AudioCommand>) -> Self {
        let lang  = Lang::parse(&cfg.app.lang);
        let pal   = Palette::by_name(&cfg.theme.name);
        let volume = cfg.player.volume.min(130);
        let mut tracks = vec![Vec::new(); Source::ALL.len()];
        tracks[Source::Local.index()] = Self::scan_local(&cfg.app.music_dir);
        // Expandir ~
        let music_dir = shellexpand(&cfg.app.music_dir);
        let _ = music_dir; // usado en scan_local vía cfg — provisorio
        App {
            lang, pal, cfg,
            source: Source::Local,
            sel_idx: { let mut s = ListState::default(); s.select(Some(0)); s },
            mode: Mode::Normal,
            search: String::new(),
            tracks,
            paused: false,
            play_state: PlayState::Stopped,
            volume,
            position: 0.0,
            cmd_tx,
            toast: None,
        }
    }

    /// Escanea ~/Music (o el dir configurado) buscando ficheros de audio.
    fn scan_local(dir: &str) -> Vec<Track> {
        let dir = shellexpand(dir);
        let mut out = Vec::new();
        let mut ls = match std::fs::read_dir(&dir) { Ok(d) => d, Err(_) => return out };
        let exts = ["mp3", "flac", "ogg", "m4a", "opus", "wav"];
        while let Some(Ok(e)) = ls.next() {
            let path = e.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if path.is_file() && exts.contains(&ext.as_str()) {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                out.push(Track::new(Source::Local, name, path.display().to_string()));
            }
        }
        out.sort_by(|a, b| a.title.cmp(&b.title));
        out
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), std::time::Instant::now()));
    }

    /// Lanza la pista seleccionada (Enter).
    fn play_selected(&mut self) {
        if self.search_results().is_empty() { return; }
        if let Some(i) = self.sel_idx.selected() {
            let viz = self.search_results();
            let t = viz[i].clone();
            let _ = self.cmd_tx.send(AudioCommand::Play(t));
            self.play_state = PlayState::Playing;
        }
    }

    /// IA/yt-dlp híbrido:.Topic novio — Búsqueda cuando la fuente es YT/Spotify/SC
    /// y el usuario pulsa Enter en modo Search.
    fn commit_search(&mut self) {
        let q = self.search.trim().to_string();
        if q.is_empty() { self.mode = Mode::Normal; return; }
        let src = self.source;
        match src {
            Source::YouTube => {
                // yt-dlp resuelve "ytsearch1:<query>" dentro de mpv directamente.
                let t = Track::new(Source::YouTube, q.clone(), format!("ytsearch1:{q}"));
                self.tracks[src.index()].insert(0, t);
                self.sel_idx.select(Some(0));
                self.play_selected();
            }
            Source::Spotify | Source::SoundCloud => {
                // Híbrido v1: buscar el nombre en YouTube y reproducir la solución.
                // TODO: aquí irá librespot (Spotify premium) / cookies de SC.
                tokio::spawn(hybrid_then_play(q.clone(), self.cmd_tx.clone(), src));
                self.play_state = PlayState::Playing;
            }
            Source::Local => { /* filtrar la lista persistente en local */ }
        }
        self.mode = Mode::Normal;
        self.search.clear();
    }

    /// Vistas filtradas por búsqueda (case-insensitive, subcadena).
    fn search_results(&self) -> Vec<Track> {
        let all = &self.tracks[self.source.index()];
        let q = self.search.to_lowercase();
        all.iter().filter(|t| q.is_empty() || t.title.to_lowercase().contains(&q)).cloned().collect()
    }

    /// Maneja configuración de eventos del motor (callback desde el loop).
    fn on_event(&mut self, ev: AudioEvent) {
        match ev {
            AudioEvent::Position(p) => self.position = p,
            AudioEvent::Paused(b)   => { self.paused = b; self.play_state = if b { PlayState::Paused } else { PlayState::Playing }; }
            AudioEvent::VolumeChanged(v) => self.volume = v,
            AudioEvent::Error(e)    => self.toast(e),
        }
    }
}

/// Fixture: busca en YT y reproduce inmediatamente (Spotify/SoundCloud hybrids).
async fn hybrid_then_play(
    query: String,
    tx: mpsc::UnboundedSender<AudioCommand>,
    src: Source,
) {
    if let Some(url) = hybrid_search(&query).await {
        let mut t = Track::new(src, query, url);
        t.id = uuid4();
        let _ = tx.send(AudioCommand::Play(t));
    }
}

fn uuid4() -> String { format!("{:x}", rand_u64()) }

/// Petit generador pseudo-random sin dependencia externa (solo para IDs demo).
fn rand_u64() -> u64 {
    use std::time::SystemTime;
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(42)
}

/// Expande "~" en rutas de configuración.
fn shellexpand(p: &str) -> PathBuf {
    if let Some(stripped) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() { return home.join(stripped); }
    }
    PathBuf::from(p)
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 8 — RENDERIZADO (Ratatui)
// ═══════════════════════════════════════════════════════════════════════════════

/// Logo ASCII de la app: "aspire" en arte de letras + "music" debajo.
const LOGO: &str = r#"
 █████╗ ███████╗██████╗ ██╗██████╗ ███████╗
 ██╔══██╗██╔════╝██╔══██╗██║██╔══██╗██╔════╝
 ███████║███████╗██████╔╝██║██████╔╝█████╗
 ██╔══██║╚════██║██╔═══╝ ██║██╔══██╗██╔══╝
 ██║  ██║███████║██║     ██║██║  ██║███████╗
 ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝
              ◆  M U S I C  ◆
"#;

/// Panel lateral izquierdo estilo "Neo-tree" — selector de fuentes.
fn render_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let p = app.pal;
    let items: Vec<ListItem> = Source::ALL.iter().map(|src| {
        let mut spans = Vec::new();
        if *src == app.source {
            spans.push(Span::styled("┃ ", Style::default().fg(p.purple)));
        } else {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("[{}] ", src.digit()),
            Style::default().fg(p.faint),
        ));
        spans.push(Span::styled(
            format!("{} {}", src.icon(), app.lang.t(src.key())),
            Style::default().fg(if *src == app.source { p.purple } else { p.fg })
                .add_modifier(if *src == app.source { Modifier::BOLD } else { Modifier::empty() }),
        ));
        ListItem::new(Line::from(spans))
    }).collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(p.border))
                .title(Span::styled(
                    app.lang.t("sources"),
                    Style::default().fg(p.cyan).add_modifier(Modifier::BOLD),
                ))
                .bg(p.panel),
        )
        .style(Style::default().bg(p.panel));
    f.render_widget(list, area);
}

/// Panel principal: logo + lista de pistas Ronald la fuente activa.
fn render_main(f: &mut Frame, area: Rect, app: &App) {
    let p = app.pal;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(1)])
        .split(area);

    // ····· Logo ─·····························································
    let logo = Paragraph::new(LOGO)
        .style(Style::default().fg(p.purple))
        .alignment(Alignment::Center)
        .block(
            Block::bordered()
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(p.border))
        );
    f.render_widget(logo, chunks[0]);

    // ····· Lista de pistas ─·················································
    let viz = app.search_results();
    let items: Vec<ListItem> = viz.iter().enumerate().map(|(i, t)| {
        let selected = app.sel_idx.selected().is_some_and(|s| s == i);
        let marker = if selected {
            Span::styled("▸ ", Style::default().fg(p.green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("  ", Style::default())
        };
        let num = Span::styled(format!("{:02} ", i + 1), Style::default().fg(p.faint));
        let dur = t.duration.map(format_time).unwrap_or_else(|| "--:--".to_string());
        let title = if selected {
            Span::styled(t.title.clone(), Style::default().fg(p.cyan).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(t.title.clone(), Style::default().fg(p.fg))
        };
        let artist = Span::styled(format!(" · {}", t.artist.replace(['-','_'], " ")), Style::default().fg(p.faint));
        let right  = Span::styled(format!("  [{}]", dur), Style::default().fg(p.faint));
        ListItem::new(Line::from(vec![marker, num, title, artist, right]))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Rgb(0x20, 0x22, 0x35)))
        .block(
            Block::bordered()
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(p.border))
                .title(Span::styled(
                    format!("  {} — {}  ", app.lang.t(app.source.key()), app.search),
                    Style::default().fg(p.blue),
                ))
        );
    f.render_stateful_widget(list, chunks[1], &mut {
        // Re-creamos el estado porque `render_stateful_widget` consume el estado.
        let mut st = app.sel_idx.clone();
        st
    });
}

/// Barra de progreso minimalista, línea completa ancho del terminal.
fn render_progress(f: &mut Frame, area: Rect, app: &App) {
    let p = app.pal;
    let ratio = 0.0f64; // TODO: la longitud exacta proviene de MPV metadata (options)
    let gauge = Gauge::default()
        .ratio(ratio)
        .gauge_style(Style::default().fg(p.green).bg(p.bg))
        .label(Span::styled(
            format!(" {} ", format_time(app.position as u64)),
            Style::default().fg(p.fg),
        ))
        .style(Style::default().bg(p.bg));
    f.render_widget(gauge, area);
}

/// Barra inferior estilo Lualine: segmentos con powerline separators (U+E0B4).
fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let p = app.pal;
    // Estado → color de modo, como en NeoVim Lualine
    // (verde = playing, amarillo = pausa, rojo = stop).
    let (mode_txt, mode_fg) = match app.play_state {
        PlayState::Playing => (app.lang.t("playing"), p.green),
        PlayState::Paused  => (app.lang.t("paused"),  p.yellow),
        PlayState::Stopped => (app.lang.t("stopped"), p.red),
    };
    let mode_style = Style::default().fg(p.bg).bg(mode_fg);
    let mut spans = vec![
        Span::styled(format!(" {} ", mode_txt.to_uppercase()), mode_style),
        Span::styled("", Style::default().fg(mode_fg).bg(p.panel)),
        Span::styled(
            format!(" 🎵 {} ", app.lang.t(app.source.key())),
            Style::default().fg(p.cyan).bg(p.panel),
        ),
        Span::styled("", Style::default().fg(p.panel).bg(p.panel)),
        Span::styled(
            format!(" {} ", app.lang.t("volume")),
            Style::default().fg(p.blue).bg(p.panel),
        ),
        Span::styled(
            format!(" {:>3}% ", app.volume),
            Style::default().fg(p.fg).bg(p.panel),
        ),
        Span::styled("", Style::default().fg(p.panel).bg(p.panel)),
        Span::styled(
            format!(" {} ", app.lang.t("hint")),
            Style::default().fg(p.faint).bg(p.panel),
        ),
    ];
    // Espaciadores powerline decorativos
    spans.drain(1..2); // skeleton: se agrega el separador real cuando se pula el estilo powerline
    spans.insert(1, Span::styled("", Style::default().fg(p.panel).bg(p.panel)));

    // Título a la derecha de la barra.
    let title = " aspire-music ".to_string();
    let title_w = title.chars().count();
    let left_w: usize = spans.iter().map(|s| s.width()).sum();
    let avail = area.width as usize;
    if left_w + title_w < avail {
        let pad = " ".repeat(avail - left_w - title_w);
        spans.push(Span::styled(pad, Style::default().bg(p.bg)));
    }
    spans.push(Span::styled(title, Style::default().fg(p.bg).bg(p.purple).add_modifier(Modifier::BOLD)));

    let para = Paragraph::new(Line::from(spans)).style(Style::default().bg(p.bg));
    f.render_widget(para, area);
}

/// Encapsula todo el frame del terminal a partir de App.
fn render_root(f: &mut Frame, app: &App) {
    let p = app.pal;
    let whole = f.area();
    f.render_widget(Block::default().style(Style::default().bg(p.bg)), whole);

    // Estructura vertical: cuerpo (sidebar + main) y abajo (progreso + status)
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)])
        .split(whole);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(1)])
        .split(v[0]);
    render_sidebar(f, body[0], app);
    render_main(f, body[1], app);
    render_progress(f, v[1], app);
    render_status(f, v[2], app);

    // Toast efímero arriba-derecha (errores del motor, info)
    if let Some((msg, t)) = &app.toast {
        if t.elapsed() < Duration::from_secs(4) {
            let area = Rect { x: 1, y: 0, width: 40.min(whole.width), height: 1 };
            let para = Paragraph::new(Line::from(Span::styled(
                format!(" {msg} "),
                Style::default().fg(p.yellow).bg(Color::Rgb(0x35, 0x30, 0x26)),
            )));
            f.render_widget(para, area);
        }
    }
}

fn format_time(s: u64) -> String { format!("{:02}:{:02}", s / 60, s % 60) }

// ═══════════════════════════════════════════════════════════════════════════════
// SECCIÓN 9 — BUCLE PRINCIPAL ASÍNCRONO
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Configuración
    let cfg = AppConfig::cargar();

    // 2. Canales de comunicación UI ↔ motor
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<AudioCommand>();
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<AudioEvent>();

    // 3. Lanzar el motor en segundo plano
    let cookies = {
        let p = shellexpand(&cfg.auth.yt_cookies_path);
        if p.exists() && !cfg.auth.yt_cookies_path.is_empty() { Some(p) } else { None }
    };
    let _ = spotify_get_token(&cfg.auth.spotify_client_id, &cfg.auth.spotify_client_secret).await; // stub inicial
    tokio::spawn(engine_task(cmd_rx, ev_tx, cookies));

    // 4. Setup del terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 5. Estado + loop de 60 FPS
    let mut app = App::new(cfg, cmd_tx.clone());
    let frame = Duration::from_millis(16);
    let res = run_app(&mut terminal, &mut app, &mut ev_rx, cmd_tx.clone(), frame).await;

    // 6. Restaurar terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ev_rx: &mut mpsc::UnboundedReceiver<AudioEvent>,
    cmd_tx: mpsc::UnboundedSender<AudioCommand>,
    frame: Duration,
) -> Result<()> {
    loop {
        // a) Eventos del motor (no bloqueante)
        while let Ok(ev) = ev_rx.try_recv() { app.on_event(ev); }

        // b) Teclado (poll corto para mantener 60 FPS)
        if event::poll(Duration::from_millis(8))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if on_key(app, &cmd_tx, key).await { break; } // true = salir
                }
            }
        }

        // c) Render a framerate fijo
        terminal.draw(|f| render_root(f, app))?;
        tokio::time::sleep(frame).await;
    }
    // Apagamos el motor limpiamente (mpv muere con ello)
    let _ = cmd_tx.send(AudioCommand::Shutdown);
    Ok(())
}

/// Traduce teclas a acciones de App. Devuelve `true` para salir.
async fn on_key(
    app: &mut App,
    tx: &mpsc::UnboundedSender<AudioCommand>,
    key: crossterm::event::KeyEvent,
) -> bool {
    let p = app.pal; _ = p;
    match app.mode {
        // ───── MODO SEARCH ─────
        Mode::Search => match key.code {
            KeyCode::Esc           => { app.mode = Mode::Normal; app.search.clear(); }
            KeyCode::Enter         => app.commit_search(),
            KeyCode::Backspace     => { app.search.pop(); }
            KeyCode::Char(c)       => app.search.push(c),
            _ => {}
        },
        // ───── MODO NORMAL ─────
        Mode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('1') => { app.source = Source::Local; }
            KeyCode::Char('2') => { app.source = Source::YouTube; }
            KeyCode::Char('3') => { app.source = Source::Spotify; }
            KeyCode::Char('4') => { app.source = Source::SoundCloud; }
            KeyCode::Tab       => { let i = (app.source.index() + 1) % 4; app.source = Source::ALL[i]; }
            KeyCode::Down | KeyCode::Char('j') => { let len = app.search_results().len(); next(&mut app.sel_idx, len); }
            KeyCode::Up   | KeyCode::Char('k') => { let len = app.search_results().len(); prev(&mut app.sel_idx, len); }
            KeyCode::Char('/') => { app.mode = Mode::Search; }
            KeyCode::Char(' ')                               => { let _ = tx.send(AudioCommand::TogglePause); }
            KeyCode::Enter                                   => app.play_selected(),
            KeyCode::Char('s')                               => { let _ = tx.send(AudioCommand::Stop);           app.play_state = PlayState::Stopped; }
            KeyCode::Char('n')                               => next_track(app),
            KeyCode::Char('p')                               => prev_track(app),
            KeyCode::Char('+') | KeyCode::Char('=')          => { app.volume = (app.volume + 5).min(130); let _ = tx.send(AudioCommand::SetVolume(app.volume)); }
            KeyCode::Char('-')                               => { app.volume = app.volume.saturating_sub(5);      let _ = tx.send(AudioCommand::SetVolume(app.volume)); }
            KeyCode::Right => { let _ = tx.send(AudioCommand::Seek(10)); }
            KeyCode::Left  => { let _ = tx.send(AudioCommand::Seek(-10)); }
            _ => {}
        },
    }
    false
}

/// Límite de loop de List: siguiente índice.
fn next(st: &mut ListState, len: usize) {
    let cur = st.selected().unwrap_or(0);
    st.select(Some(if len == 0 { 0 } else { (cur + 1).min(len - 1) }));
}
fn prev(st: &mut ListState, len: usize) {
    let cur = st.selected().unwrap_or(0);
    st.select(Some(if len == 0 { 0 } else { cur.saturating_sub(1) }));
}

/// El track siguiente de la fuente activa (para reproducir en orden).
fn next_track(app: &mut App) {
    let len = app.tracks[app.source.index()].len();
    if len == 0 { return; }
    next(&mut app.sel_idx, len);
    app.play_selected();
}
fn prev_track(app: &mut App) {
    let len = app.tracks[app.source.index()].len();
    if len == 0 { return; }
    prev(&mut app.sel_idx, len);
    app.play_selected();
}
