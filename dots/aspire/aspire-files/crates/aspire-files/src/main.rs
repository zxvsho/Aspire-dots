//! aspire-files — gestor de archivos GTK4 para Wayland.
//!
//! Interfaz fina sobre `aspire-fs`, `aspire-config` y `aspire-thumb`.
//! No depende de libadwaita ni de servicios de GNOME.

mod app;
mod dialogs;
mod theme;
mod views;

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use aspire_fs::ListOptions;
use gtk::prelude::*;

const HELP: &str = "\
aspire-files — GTK4 file manager for Wayland

Usage:
  aspire-files [DIRECTORY]

Options:
  -h, --help              Show this help
  -V, --version           Show the version
  --bench [DIR]           Measure a directory listing and exit
  --time-open [DIR]       Measure opening a directory and exit
  --stress [DIR]          Repeat the listing 50 times and report timings
  --shot FILE             Capture the screen with grim and exit
  --act open DIR          Open the graphical application at DIR
  --act new-folder DIR    Create a folder named \"New Folder\" in DIR

Keyboard:
  Ctrl+L location · Ctrl+F search · Ctrl+Shift+F global search
  Ctrl+1 grid · Ctrl+2 list · Ctrl+T new tab · Ctrl+W close tab
  Ctrl+Shift+N new folder · F2 rename · Delete move to trash
  Shift+Delete delete permanently · F5 refresh · F9 sidebar
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("aspire-files {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|arg| arg == "--bench") {
        bench(value_after(&args, "--bench").unwrap_or_else(|| ".".into()));
        return;
    }
    if args.iter().any(|arg| arg == "--time-open") {
        bench(value_after(&args, "--time-open").unwrap_or_else(|| ".".into()));
        return;
    }
    if args.iter().any(|arg| arg == "--stress") {
        stress(value_after(&args, "--stress").unwrap_or_else(|| ".".into()));
        return;
    }
    if let Some(index) = args.iter().position(|arg| arg == "--shot") {
        let Some(file) = args.get(index + 1) else {
            eprintln!("--shot requires a file name");
            std::process::exit(2);
        };
        shot(file);
        return;
    }
    if let Some(index) = args.iter().position(|arg| arg == "--act") {
        let action = args.get(index + 1).map(String::as_str).unwrap_or("");
        let directory = args.get(index + 2).map(PathBuf::from);
        match action {
            "new-folder" => {
                let parent = directory.unwrap_or_else(aspire_fs::home_dir);
                match aspire_fs::create_dir(&parent, "New Folder") {
                    Ok(path) => println!("{}", path.display()),
                    Err(err) => {
                        eprintln!("cannot create folder: {err}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            "open" => {
                launch_gui(directory);
                return;
            }
            _ => {
                eprintln!("unknown --act value: {action} (use open or new-folder)");
                std::process::exit(2);
            }
        }
    }

    let initial = args
        .into_iter()
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from);
    launch_gui(initial);
}

fn value_after(args: &[String], flag: &str) -> Option<PathBuf> {
    let index = args.iter().position(|arg| arg == flag)?;
    let value = args.get(index + 1)?;
    if value.starts_with('-') {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

fn launch_gui(initial: Option<PathBuf>) {
    gtk::glib::set_application_name("Aspire Files");
    let application = gtk::Application::builder()
        .application_id("io.github.zxvsho.AspireFiles")
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    application.connect_activate(move |application| {
        gtk::Window::set_default_icon_name("system-file-manager");
        let _app = app::build(application, initial.clone());
    });
    // No pasamos argv a GApplication: la ruta inicial se entrega por closure.
    application.run_with_args(&["aspire-files"]);
}

fn bench(directory: PathBuf) {
    let start = Instant::now();
    match aspire_fs::list_dir(&directory, ListOptions::default()) {
        Ok(entries) => {
            let elapsed = start.elapsed();
            println!(
                "{} entries in {} — {:.2} ms",
                entries.len(),
                directory.display(),
                elapsed.as_secs_f64() * 1000.0
            );
        }
        Err(err) => {
            eprintln!("cannot list {}: {err}", directory.display());
            std::process::exit(1);
        }
    }
}

fn stress(directory: PathBuf) {
    let mut worst = 0.0_f64;
    let mut total = 0.0_f64;
    for _ in 0..50 {
        let start = Instant::now();
        let Ok(_) = aspire_fs::list_dir(&directory, ListOptions::default()) else {
            eprintln!("cannot list {}", directory.display());
            std::process::exit(1);
        };
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        worst = worst.max(elapsed);
        total += elapsed;
    }
    println!(
        "50 listings of {} — avg {:.2} ms, worst {:.2} ms",
        directory.display(),
        total / 50.0,
        worst
    );
}

fn shot(file: &str) {
    match Command::new("grim").arg(file).status() {
        Ok(status) if status.success() => println!("screenshot written to {file}"),
        Ok(_) => {
            eprintln!("grim failed");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("cannot run grim: {err}");
            std::process::exit(1);
        }
    }
}
