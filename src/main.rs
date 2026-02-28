#![windows_subsystem = "windows"]

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use notify_rust::Notification;
use serde::Deserialize;
use std::fs;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use windows::Win32::{
    Foundation::BOOL,
    Media::Audio::{
        Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
        eConsole,
    },
    System::Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx},
    UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage},
};

#[derive(Deserialize)]
struct Settings {
    #[serde(default = "default_hotkey")]
    hotkey: String,
}

fn default_hotkey() -> String {
    "ctrl+shift+m".to_string()
}

fn load_settings() -> Settings {
    let path = dirs::home_dir()
        .unwrap_or_default()
        .join(".muter")
        .join("settings.toml");
    match fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).unwrap_or(Settings {
            hotkey: default_hotkey(),
        }),
        Err(_) => {
            let settings = Settings {
                hotkey: default_hotkey(),
            };
            let content = format!(
                "# Muter settings\n\
                 #\n\
                 # Hotkey to toggle microphone mute\n\
                 # Modifiers: ctrl, shift, alt, super\n\
                 # Keys: a-z, 0-9, F1-F12, Space, Tab, etc.\n\
                 #\n\
                 # Examples:\n\
                 #   hotkey = \"ctrl+shift+m\"\n\
                 #   hotkey = \"ctrl+alt+m\"\n\
                 #   hotkey = \"super+shift+a\"\n\
                 #   hotkey = \"ctrl+F9\"\n\
                 \n\
                 hotkey = \"{}\"\n",
                settings.hotkey
            );
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&path, content);
            settings
        }
    }
}

fn render_icon(muted: bool) -> Icon {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let mic_color: [u8; 4] = if muted {
        [160, 160, 160, 255]
    } else {
        [255, 255, 255, 255]
    };

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let idx = ((y * size + x) * 4) as usize;

            // Microphone capsule (vertical pill shape)
            // Segment from (16, 6) to (16, 12), radius 4.5
            let dx = fx - 16.0;
            let clamped_y = fy.clamp(6.0, 12.0);
            let seg_dist = (dx * dx + (fy - clamped_y).powi(2)).sqrt();
            let in_capsule = seg_dist <= 4.5;

            // Holder arc (U-shape around capsule bottom)
            let arc_dist = ((fx - 16.0).powi(2) + (fy - 12.0).powi(2)).sqrt();
            let in_holder = fy >= 12.0 && (5.5..=7.5).contains(&arc_dist);

            // Stand (vertical bar)
            let in_stand = (fx - 16.0).abs() <= 1.5 && (19.5..=25.0).contains(&fy);

            // Base (horizontal bar)
            let in_base = (10.0..=22.0).contains(&fx) && (25.0..=27.0).contains(&fy);

            if in_capsule || in_holder || in_stand || in_base {
                rgba[idx..idx + 4].copy_from_slice(&mic_color);
            }

            // Prohibition overlay when muted
            if muted {
                let cx = fx - 16.0;
                let cy = fy - 16.0;
                let dist = (cx * cx + cy * cy).sqrt();

                let red = [220u8, 30, 30, 255];

                // Red circle outline
                if (dist - 14.0).abs() <= 1.5 {
                    rgba[idx..idx + 4].copy_from_slice(&red);
                }

                // Diagonal slash (upper-left to lower-right)
                let line_dist = (cx - cy).abs() / 1.414;
                if line_dist <= 1.5 && dist <= 14.0 {
                    rgba[idx..idx + 4].copy_from_slice(&red);
                }
            }
        }
    }

    Icon::from_rgba(rgba, size, size).unwrap()
}

struct Icons {
    unmuted: Icon,
    muted: Icon,
}

fn get_mic_endpoint_volume() -> IAudioEndpointVolume {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .unwrap();
        device.Activate(CLSCTX_ALL, None).unwrap()
    }
}

fn toggle_mute(ep: &IAudioEndpointVolume) -> bool {
    unsafe {
        let muted = ep.GetMute().unwrap().as_bool();
        let new_state = !muted;
        ep.SetMute(BOOL(new_state as i32), std::ptr::null())
            .unwrap();
        new_state
    }
}

fn do_toggle(ep: &IAudioEndpointVolume, tray: &TrayIcon, icons: &Icons) {
    let muted = toggle_mute(ep);
    let icon = if muted { &icons.muted } else { &icons.unmuted };
    tray.set_icon(Some(icon.clone())).unwrap();
    let body = if muted {
        "Microphone Muted"
    } else {
        "Microphone Unmuted"
    };
    std::thread::spawn(move || {
        let _ = Notification::new().summary("Muter").body(body).show();
    });
}

fn main() {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).unwrap() };

    let endpoint = get_mic_endpoint_volume();
    let muted = unsafe { endpoint.GetMute().unwrap().as_bool() };

    let icons = Icons {
        unmuted: render_icon(false),
        muted: render_icon(true),
    };

    let menu = Menu::new();
    let toggle_item = MenuItem::new("Toggle Mute", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    menu.append(&toggle_item).unwrap();
    menu.append(&quit_item).unwrap();

    let initial_icon = if muted { &icons.muted } else { &icons.unmuted };
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Muter")
        .with_icon(initial_icon.clone())
        .build()
        .unwrap();

    let settings = load_settings();
    let _manager = GlobalHotKeyManager::new().unwrap();
    let hotkey: HotKey = settings
        .hotkey
        .parse()
        .expect("Invalid hotkey in settings.toml");
    _manager.register(hotkey).unwrap();

    let menu_rx = MenuEvent::receiver();
    let hotkey_rx = GlobalHotKeyEvent::receiver();
    let toggle_id = toggle_item.id();
    let quit_id = quit_item.id();

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            if let Ok(ev) = hotkey_rx.try_recv()
                && ev.state == HotKeyState::Pressed
            {
                do_toggle(&endpoint, &tray, &icons);
            }

            if let Ok(ev) = menu_rx.try_recv() {
                if ev.id() == toggle_id {
                    do_toggle(&endpoint, &tray, &icons);
                } else if ev.id() == quit_id {
                    break;
                }
            }
        }
    }
}
