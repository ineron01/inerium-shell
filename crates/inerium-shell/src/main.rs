use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use gdk4::prelude::*;
use gdk4::Key;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, EventControllerKey};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use serde_json::Value;

mod bg_generator;
mod hw;
mod theme;
mod views;

macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) || std::env::var_os("INERIUM_DEBUG").is_some() {
            eprintln!($($arg)*);
        }
    };
}

fn get_sock_path() -> PathBuf {
    inerium_tools::sys::runtime_dir().join("inerium-popups.sock")
}

fn get_cursor_pos() -> (i32, i32) {
    if let Ok(v) = inerium_tools::ipc::hyprland::query_json::<Value>("cursorpos")
        && let (Some(x), Some(y)) = (v["x"].as_i64(), v["y"].as_i64())
    {
        return (x as i32, y as i32);
    }
    (0, 0)
}

fn get_cursor_and_monitor() -> (i32, i32, i32, i32, i32, i32, String) {
    inerium_tools::ipc::hyprland::get_cursor_and_monitor()
        .unwrap_or((960, 540, 0, 0, 1920, 1080, String::new()))
}

struct SafeZone {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    pad: i32,
    opened_at: std::time::Instant,
    has_entered: bool,
    mx: i32,
    my: i32,
    mw: i32,
    mh: i32,
    outside_since: Option<std::time::Instant>,
}

struct AppState {
    app: Option<Application>,
    window: Option<ApplicationWindow>,
    active_controller: Option<Box<dyn std::any::Any>>,
    current_view: String,
    is_visible: bool,
    safe_zone: Option<SafeZone>,
    session_id: u64,
    modal_dialog_active: bool,
    css_provider: Option<CssProvider>,
}

thread_local! {
    static STATE: RefCell<AppState> = const { RefCell::new(AppState {
        app: None,
        window: None,
        active_controller: None,
        current_view: String::new(),
        is_visible: false,
        safe_zone: None,
        session_id: 0,
        modal_dialog_active: false,
        css_provider: None,
    }) };
}

pub fn set_modal_dialog_active(active: bool) {
    STATE.with(|s| {
        s.borrow_mut().modal_dialog_active = active;
    });
}

static CMD_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn main() {
    let args: Vec<String> = env::args().collect();

    // Client mode: if passed a command and not --daemon, send to daemon socket and exit
    if args.len() > 1 && args[1] != "--daemon" {
        let sock = get_sock_path();
        if let Ok(mut stream) = UnixStream::connect(&sock) {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(500)));
            let msg = format!("{}\n", args[1..].join(" "));
            let _ = stream.write_all(msg.as_bytes());
            let mut buf = [0u8; 16];
            let _ = stream.read(&mut buf);
            return;
        }
    }

    // Daemon mode: start GTK layer-shell application
    // Isolate shell styling from external GTK_THEME overrides
    unsafe { env::remove_var("GTK_THEME"); }

    let app = Application::builder()
        .application_id("org.inerium.shell")
        .build();

    app.connect_startup(move |app| {
        let cur_theme = theme::detect_theme();
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(cur_theme == theme::ThemeMode::Dark);
        }
        // Load global Inerium CSS with priority above USER to guarantee pristine styling
        let provider = CssProvider::new();
        provider.load_from_data(&theme::get_theme_css(cur_theme));
        if let Some(display) = gdk4::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER + 100,
            );
        }

        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.app = Some(app.clone());
            st.css_provider = Some(provider);
        });
    });

    app.connect_activate(move |app| {
        let guard = app.hold();
        Box::leak(Box::new(guard)); // Keep daemon held permanently

        // Initialize Night Light state/schedule and sync every 5s (fast minute-boundary response)
        hw::night_light::init_on_startup();
        glib::timeout_add_seconds_local(5, || {
            hw::night_light::sync_schedule_tick();
            glib::ControlFlow::Continue
        });

        // Watch ~/.config directory for locale.conf changes (resilient to atomic fs::rename & inode recreation)
        if let Some(home) = env::var_os("HOME") {
            let config_dir = PathBuf::from(home).join(".config");
            let gdir = gio::File::for_path(&config_dir);
            if let Ok(monitor) = gdir.monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) {
                monitor.connect_changed(|_, file, other_file, event| {
                    let is_target = file.basename().is_some_and(|b| b == Path::new("locale.conf"))
                        || other_file
                            .as_ref()
                            .and_then(|f| f.basename())
                            .is_some_and(|b| b == Path::new("locale.conf"));

                    if is_target
                        && matches!(
                            event,
                            gio::FileMonitorEvent::ChangesDoneHint
                                | gio::FileMonitorEvent::Created
                                | gio::FileMonitorEvent::MovedIn
                                | gio::FileMonitorEvent::Renamed
                                | gio::FileMonitorEvent::Changed
                        )
                    {
                        let old_lang = inerium_i18n::Lang::get();
                        inerium_i18n::Lang::reinit();
                        let new_lang = inerium_i18n::Lang::get();
                        if old_lang != new_lang {
                            let (is_vis, cur) = STATE.with(|s| {
                                let st = s.borrow();
                                (st.is_visible && !st.current_view.is_empty(), st.current_view.clone())
                            });
                            if is_vis {
                                show_view_and_cleanup(&cur, true);
                            }
                        }
                    }
                });
                Box::leak(Box::new(monitor));
            }
        }

        let sock_path = get_sock_path();
        if sock_path.exists() {
            if UnixStream::connect(&sock_path).is_ok() {
                eprintln!("inerium-shell daemon is already running on {}", sock_path.display());
                std::process::exit(0);
            }
            let _ = fs::remove_file(&sock_path);
        }

        // Start Unix domain socket listener thread
        thread::spawn(move || {
            let listener = match UnixListener::bind(&sock_path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to bind socket: {}", e);
                    return;
                }
            };

            for mut s in listener.incoming().flatten() {
                let timeout = Some(Duration::from_secs(2));
                let _ = s.set_read_timeout(timeout);
                let _ = s.set_write_timeout(timeout);
                let mut reader = BufReader::new(&mut s);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    let cmd = line.trim().to_string();
                    if let Ok(mut q) = CMD_QUEUE.lock() {
                        q.push(cmd);
                    }

                    glib::MainContext::default().invoke(|| {
                        let cmds: Vec<String> = if let Ok(mut q) = CMD_QUEUE.lock() {
                            q.drain(..).collect()
                        } else {
                            Vec::new()
                        };

                        for c in cmds {
                            handle_command(&c);
                        }
                    });

                    let _ = s.write_all(b"OK\n");
                }
            }
        });
    });

    app.run_with_args(&["inerium-shell"]);
}

fn cleanup_displaced(win: Option<ApplicationWindow>, ctrl: Option<Box<dyn std::any::Any>>) {
    if let Some(w) = win {
        w.close();
    }
    drop(ctrl);
}

fn hide_window(state: &mut AppState) -> (Option<ApplicationWindow>, Option<Box<dyn std::any::Any>>) {
    let win = state.window.take();
    let ctrl = state.active_controller.take();
    state.is_visible = false;
    state.current_view.clear();
    state.safe_zone = None;
    state.modal_dialog_active = false;
    (win, ctrl)
}

fn hide_window_and_cleanup() {
    let (w, c) = STATE.with(|s| hide_window(&mut s.borrow_mut()));
    cleanup_displaced(w, c);
}

fn show_view_and_cleanup(view_name: &str, preserve_position: bool) {
    let (w, c) = STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.current_view = view_name.to_string();
        show_view(&mut st, view_name, preserve_position)
    });
    cleanup_displaced(w, c);
}

fn handle_command(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let target = parts.first().copied().unwrap_or("control").to_lowercase();

    if target == "close" {
        hide_window_and_cleanup();
        return;
    }

    if target == "theme" && parts.len() > 1 {
        let mode = if parts.get(1).map(|s| *s == "light").unwrap_or(false) {
            theme::ThemeMode::Light
        } else if parts.get(1).map(|s| *s == "dark").unwrap_or(false) {
            theme::ThemeMode::Dark
        } else {
            theme::detect_theme()
        };
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(mode == theme::ThemeMode::Dark);
        }
        let (is_vis, cur_view) = STATE.with(|s| {
            let state = s.borrow();
            if let Some(provider) = &state.css_provider {
                provider.load_from_data(&theme::get_theme_css(mode));
            }
            (state.is_visible, state.current_view.clone())
        });
        if is_vis && !cur_view.is_empty() {
            show_view_and_cleanup(&cur_view, true);
        }
        return;
    }

    let should_hide = STATE.with(|s| {
        let state = s.borrow();
        state.is_visible && state.current_view == target
    });

    if should_hide {
        hide_window_and_cleanup();
        return;
    }

    show_view_and_cleanup(&target, false);
}

fn show_view(state: &mut AppState, view_name: &str, preserve_position: bool) -> (Option<ApplicationWindow>, Option<Box<dyn std::any::Any>>) {
    let old_win = state.window.take();
    let old_ctrl = state.active_controller.take();

    if !preserve_position {
        state.modal_dialog_active = false;
    }

    let app = match state.app.as_ref() {
        Some(a) => a.clone(),
        None => return (old_win, old_ctrl),
    };

    let (cx, cy, mx, my, mw, mh, m_name) = get_cursor_and_monitor();
    log_debug!("show_view: view={}, m_name={}, cx={}, cy={}, mx={}, my={}, mw={}, mh={}", view_name, m_name, cx, cy, mx, my, mw, mh);

    let win = ApplicationWindow::builder()
        .application(&app)
        .css_classes(["popup-window"])
        .build();

    win.init_layer_shell();
    if view_name == "power" {
        win.set_namespace(Some("inerium-power"));
        win.add_css_class("power-window");
    } else {
        win.set_namespace(Some("org.inerium.popups.daemon"));
    }
    win.set_layer(Layer::Overlay);

    // Assign window to the active GdkMonitor
    if let Some(display) = gdk4::Display::default() {
        let gdk_mons = display.monitors();
        for i in 0..gdk_mons.n_items() {
            if let Some(m) = gdk_mons.item(i).and_downcast::<gdk4::Monitor>()
                && m.connector().as_deref() == Some(&m_name) {
                    win.set_monitor(Some(&m));
                    break;
                }
        }
    }

    // Attach Escape key controller
    let key_ctrl = EventControllerKey::new();
    key_ctrl.connect_key_pressed(|_, key, _, _| {
        if key == Key::Escape {
            hide_window_and_cleanup();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    win.add_controller(key_ctrl);

    let nav_cb = move |next_view: &str| {
        show_view_and_cleanup(next_view, true);
    };

    let close_cb = move || {
        hide_window_and_cleanup();
    };

    let (child, req_w, req_h, new_ctrl) = match view_name {
        "audio" => views::audio::build_audio_view(nav_cb, close_cb),
        "bluetooth" => views::bluetooth::build_bluetooth_view(nav_cb, close_cb),
        "network" => views::network::build_network_view(nav_cb, close_cb),
        "media" => views::media::build_media_view(nav_cb, close_cb),
        "power" => views::power::build_power_menu(close_cb),
        "appearance" | "wallpaper" | "themes" | "theme" => {
            views::appearance::build_appearance_view(nav_cb, close_cb, Some(m_name), false)
        }
        "appearance:night-light" => {
            views::appearance::build_appearance_view(nav_cb, close_cb, Some(m_name), true)
        }
        _ => views::control_center::build_control_center(nav_cb, close_cb),
    };

    let max_h = mh - 44;
    let effective_h = req_h.min(max_h);

    let (win_global_x, win_global_y) = if view_name == "power" {
        win.set_keyboard_mode(KeyboardMode::Exclusive);
        win.set_exclusive_zone(-1);
        win.set_anchor(Edge::Top, true);
        win.set_anchor(Edge::Bottom, true);
        win.set_anchor(Edge::Left, true);
        win.set_anchor(Edge::Right, true);
        win.set_child(Some(&child));
        (mx, my)
    } else {
        child.set_size_request(req_w, effective_h);
        win.set_size_request(req_w, effective_h);
        win.set_default_size(req_w, effective_h);
        win.set_resizable(false);
        win.set_child(Some(&child));

        win.set_keyboard_mode(KeyboardMode::OnDemand);
        let max_margin_left = (mw - req_w - 8).max(8);
        let margin_left = if preserve_position && state.is_visible {
            if let Some(sz) = &state.safe_zone {
                (sz.x - mx).clamp(8, max_margin_left)
            } else {
                max_margin_left
            }
        } else {
            let rel_x = cx - mx;
            if rel_x > (mw * 3 / 4) {
                max_margin_left
            } else {
                (rel_x - (req_w / 2)).clamp(8, max_margin_left)
            }
        };

        win.set_anchor(Edge::Top, true);
        win.set_anchor(Edge::Left, true);
        win.set_anchor(Edge::Bottom, false);
        win.set_anchor(Edge::Right, false);
        win.set_margin(Edge::Top, 4);
        win.set_margin(Edge::Left, margin_left);
        (mx + margin_left, my + 40)
    };

    let pad = 65;
    let x_min_init = win_global_x - pad;
    let x_max_init = win_global_x + req_w + pad;
    let y_min_init = my;
    let y_max_init = win_global_y + effective_h + pad;
    let init_inside = cx >= x_min_init && cx <= x_max_init && cy >= y_min_init && cy <= y_max_init;

    state.session_id = state.session_id.wrapping_add(1);
    let current_session = state.session_id;

    state.safe_zone = Some(SafeZone {
        x: win_global_x,
        y: win_global_y,
        w: req_w,
        h: effective_h,
        pad,
        opened_at: std::time::Instant::now(),
        has_entered: init_inside,
        mx,
        my,
        mw,
        mh,
        outside_since: None,
    });

    win.present();
    state.window = Some(win);
    state.active_controller = Some(new_ctrl);
    state.is_visible = true;

    // Start Safe Zone watcher timer (runs only while is_visible and session matches)
    if view_name != "power" {
        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            let res = STATE.with(|s| {
                let mut st = s.borrow_mut();
                if !st.is_visible || st.session_id != current_session {
                    return (glib::ControlFlow::Break, None);
                }

                let (alloc_w, alloc_h) = if let Some(ref w) = st.window {
                    (w.allocated_width(), w.allocated_height())
                } else {
                    (0, 0)
                };

                let is_modal = st.modal_dialog_active;

                if let Some(ref mut sz) = st.safe_zone {
                    if alloc_w > 50 {
                        sz.w = alloc_w;
                    }
                    if alloc_h > 50 {
                        sz.h = alloc_h;
                    }

                    let (cx, cy) = get_cursor_pos();

                    // If cursor moved to a different monitor, allow closing even if modal_dialog_active was set
                    let cursor_on_other_monitor = cx < sz.mx || cx >= sz.mx + sz.mw || cy < sz.my || cy >= sz.my + sz.mh;
                    if is_modal && !cursor_on_other_monitor {
                        return (glib::ControlFlow::Continue, None);
                    }

                    let x_min = sz.x - sz.pad;
                    let x_max = sz.x + sz.w + sz.pad;
                    let y_min = sz.y - sz.pad - 40; // include top bar area
                    let y_max = sz.y + sz.h + sz.pad;

                    let is_inside = cx >= x_min && cx <= x_max && cy >= y_min && cy <= y_max;

                    if is_inside {
                        sz.has_entered = true;
                        sz.outside_since = None;
                    } else if sz.has_entered && sz.opened_at.elapsed() > std::time::Duration::from_millis(250) {
                        let since = sz.outside_since.get_or_insert_with(std::time::Instant::now);
                        if since.elapsed() > std::time::Duration::from_millis(180) {
                            log_debug!("SafeZone closed: cx={}, cy={}, bounds=[{}, {}]x[{}, {}]", cx, cy, x_min, x_max, y_min, y_max);
                            let (w, c) = hide_window(&mut st);
                            return (glib::ControlFlow::Break, Some((w, c)));
                        }
                    }
                }

                (glib::ControlFlow::Continue, None)
            });

            if let Some((w, c)) = res.1 {
                cleanup_displaced(w, c);
            }
            res.0
        });
    }

    (old_win, old_ctrl)
}
