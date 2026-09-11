#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn data_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("quicktimer");
    let _ = fs::create_dir_all(&path);
    path
}

#[tauri::command]
fn save_json(key: String, value: serde_json::Value) -> Result<(), String> {
    let mut path = data_dir();
    path.push(format!("{}.json", key));
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_json(key: String) -> Result<serde_json::Value, String> {
    let mut path = data_dir();
    path.push(format!("{}.json", key));
    if !path.exists() {
        return Ok(serde_json::Value::Null);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
async fn show_popup(app: tauri::AppHandle, duration: u64) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("popup") {
        let _ = existing.close();
    }
    std::thread::sleep(std::time::Duration::from_millis(60));
    let url = WebviewUrl::App(format!("popup.html?d={}", duration).into());
    let popup = WebviewWindowBuilder::new(&app, "popup", url)
        .title("Quick Reset")
        .inner_size(360.0, 230.0)
        .always_on_top(true)
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = popup.show();
    Ok(())
}

#[tauri::command]
fn popup_closed(app: tauri::AppHandle) {
    let _ = app.emit("popup-closed", ());
}

#[tauri::command]
fn hide_popup(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.close();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let target = Shortcut::new(
                            Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
                            Code::KeyQ,
                        );
                        if shortcut == &target {
                            let _ = app.emit("test-popup", ());
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let show_i = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
            let test_i = MenuItem::with_id(app, "test", "Test Popup", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &test_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                    "test" => {
                        let _ = app.emit("test-popup", ());
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let shortcut = Shortcut::new(
                Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
                Code::KeyQ,
            );
            app.global_shortcut().register(shortcut)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_json,
            load_json,
            show_popup,
            popup_closed,
            hide_popup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}