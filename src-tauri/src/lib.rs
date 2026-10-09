//! Tauri shell: owns the decoding pipeline, runs capture (or the demo), and pushes snapshots to both windows.

mod capture;
mod settings;

use capture::CaptureStatus;
use meter_core::combat::{Snapshot, TrackerOptions};
use meter_core::demo::Demo;
use meter_core::pipeline::Pipeline;
use parking_lot::Mutex;
use serde::Serialize;
use settings::Settings;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub struct Shared {
    pub pipeline: Mutex<Pipeline>,
    status: Mutex<CaptureStatus>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    demo: Mutex<Option<Demo>>,
}

impl Shared {
    pub fn set_status(&self, s: CaptureStatus) {
        *self.status.lock() = s;
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Update {
    status: CaptureStatus,
    current: Option<Snapshot>,
    history_len: usize,
    self_name: Option<String>,
    settings: Settings,
}

fn tracker_options(s: &Settings) -> TrackerOptions {
    TrackerOptions { idle_timeout_ms: s.idle_timeout_s.max(1) * 1000, ..Default::default() }
}

fn filtered(mut snap: Snapshot, players_only: bool) -> Snapshot {
    if players_only && snap.actors.iter().any(|a| a.is_player) {
        snap.actors.retain(|a| a.is_player);
    }
    snap
}

fn update(shared: &Shared) -> Update {
    let settings = shared.settings.lock().clone();
    let pipe = shared.pipeline.lock();
    Update {
        status: if settings.demo { CaptureStatus::Off } else { shared.status.lock().clone() },
        current: pipe.tracker.snapshot().map(|s| filtered(s, settings.players_only)),
        history_len: pipe.tracker.history.len(),
        self_name: pipe.tracker.self_name().map(str::to_owned),
        settings,
    }
}

/// (Re)starts whatever feeds the pipeline: the demo, or live capture.
fn restart_source(shared: &Arc<Shared>) {
    let settings = shared.settings.lock().clone();
    *shared.pipeline.lock() = Pipeline::new(settings.game_ports.clone(), tracker_options(&settings));
    if settings.demo {
        capture::stop();
        let demo = Demo::new(now_ms());
        {
            let mut pipe = shared.pipeline.lock();
            for ev in demo.intro() {
                pipe.tracker.event(&ev);
            }
            pipe.tracker.set_name(Demo::BOSS_ID, Demo::BOSS_NAME);
        }
        // Lock order everywhere is demo before pipeline, so the pipeline lock above is released first.
        *shared.demo.lock() = Some(demo);
    } else {
        *shared.demo.lock() = None;
        capture::start(shared.clone(), &settings.game_ports);
    }
}

fn apply_overlay(app: &AppHandle, s: &Settings) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.set_ignore_cursor_events(s.overlay_locked);
        let _ = if s.overlay_visible { w.show() } else { w.hide() };
    }
}

fn change_settings(app: &AppHandle, f: impl FnOnce(&mut Settings)) -> Settings {
    let shared = app.state::<Arc<Shared>>();
    let (before, after) = {
        let mut s = shared.settings.lock();
        let before = s.clone();
        f(&mut s);
        s.save(&shared.settings_path);
        (before, s.clone())
    };
    if before.demo != after.demo || before.game_ports != after.game_ports || before.idle_timeout_s != after.idle_timeout_s {
        restart_source(&shared);
    }
    apply_overlay(app, &after);
    let _ = app.emit("meter://update", update(&shared));
    after
}

#[tauri::command]
fn get_update(shared: State<Arc<Shared>>) -> Update {
    update(&shared)
}

#[tauri::command]
fn get_history(shared: State<Arc<Shared>>) -> Vec<Snapshot> {
    let players_only = shared.settings.lock().players_only;
    shared.pipeline.lock().tracker.history.iter().cloned().map(|s| filtered(s, players_only)).collect()
}

#[tauri::command]
fn reset_encounter(shared: State<Arc<Shared>>) {
    shared.pipeline.lock().tracker.finish();
}

#[tauri::command]
fn set_settings(app: AppHandle, settings: Settings) -> Settings {
    change_settings(&app, |s| *s = settings)
}

#[tauri::command]
fn retry_capture(shared: State<Arc<Shared>>) {
    restart_source(&shared);
}

pub fn run() {
    let lock = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL);
    let toggle = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyO);
    let reset = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut == &lock {
                        change_settings(app, |s| s.overlay_locked = !s.overlay_locked);
                    } else if shortcut == &toggle {
                        change_settings(app, |s| s.overlay_visible = !s.overlay_visible);
                    } else if shortcut == &reset {
                        app.state::<Arc<Shared>>().pipeline.lock().tracker.finish();
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = Settings::load(&settings_path);
            let shared = Arc::new(Shared {
                pipeline: Mutex::new(Pipeline::new(settings.game_ports.clone(), tracker_options(&settings))),
                status: Mutex::new(CaptureStatus::Off),
                settings: Mutex::new(settings.clone()),
                settings_path,
                demo: Mutex::new(None),
            });
            app.manage(shared.clone());
            restart_source(&shared);
            apply_overlay(app.handle(), &settings);

            for s in [lock, toggle, reset] {
                if let Err(e) = app.global_shortcut().register(s) {
                    eprintln!("could not register shortcut {s:?}: {e}");
                }
            }

            let show = MenuItem::with_id(app, "show", "Open meter", true, None::<&str>)?;
            let overlay = MenuItem::with_id(app, "overlay", "Show / hide overlay", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &overlay, &quit])?;
            let mut tray = TrayIconBuilder::new().menu(&menu).tooltip("AION 2 Meter").on_menu_event(|app, e| {
                match e.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "overlay" => {
                        change_settings(app, |s| s.overlay_visible = !s.overlay_visible);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                }
            });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // Four updates a second to both windows; also drives the demo and closes idle encounters.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(250));
                let now = now_ms();
                {
                    let mut demo = shared.demo.lock();
                    let mut pipe = shared.pipeline.lock();
                    if let Some(d) = demo.as_mut() {
                        // Fight for 40 s, pause for 12 s, repeat, so the history fills up too.
                        let fighting = (d.now_ms() / 1000) % 52 < 40;
                        let events = d.step(250);
                        if fighting {
                            for ev in events {
                                pipe.tracker.event(&ev);
                            }
                        }
                    }
                    pipe.tick(now);
                }
                let _ = handle.emit("meter://update", update(&shared));
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window quits the app (otherwise the overlay would linger with no way back).
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { .. } = event {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![get_update, get_history, reset_encounter, set_settings, retry_capture])
        .run(tauri::generate_context!())
        .expect("error while running the app");
}
