use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{Align, GestureClick, Orientation};
use inerium_i18n::tr;
use relm4::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub player_name: String,
    pub art_url: String,
    pub position_str: String,
    pub length_str: String,
    pub position_us: u64,
    pub length_us: u64,
    pub is_playing: bool,
}

pub fn query_media_info() -> Option<MediaInfo> {
    let out = Command::new("playerctl")
        .args([
            "metadata",
            "--format",
            "{{title}}@|@{{artist}}@|@{{album}}@|@{{mpris:artUrl}}@|@{{duration(position)}}@|@{{duration(mpris:length)}}@|@{{status}}@|@{{playerName}}@|@{{position}}@|@{{mpris:length}}",
        ])
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }

    let parts: Vec<&str> = text.split("@|@").collect();
    if parts.len() < 7 {
        return None;
    }

    let player_name = if parts.len() > 7 {
        parts[7].trim().to_string()
    } else {
        String::new()
    };
    let position_us = if parts.len() > 8 {
        parts[8].trim().parse::<u64>().unwrap_or(0)
    } else {
        0
    };
    let length_us = if parts.len() > 9 {
        parts[9].trim().parse::<u64>().unwrap_or(0)
    } else {
        0
    };

    Some(MediaInfo {
        title: parts[0].trim().to_string(),
        artist: parts[1].trim().to_string(),
        album: parts[2].trim().to_string(),
        art_url: parts[3].trim().to_string(),
        position_str: parts[4].trim().to_string(),
        length_str: parts[5].trim().to_string(),
        is_playing: parts[6].trim().eq_ignore_ascii_case("playing"),
        player_name,
        position_us,
        length_us,
    })
}

fn url_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            result.push(val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn get_art_cache_path(url: &str) -> (PathBuf, PathBuf) {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    let target = runtime_dir.join(format!("inerium-art-{:x}.png", hash));
    let tmp = runtime_dir.join(format!("inerium-art-{:x}.tmp", hash));
    (target, tmp)
}

fn get_player_badge(name: &str) -> (&'static str, String) {
    let lower = name.to_lowercase();
    if lower.contains("chrom") {
        ("󰊯", "Chromium".to_string())
    } else if lower.contains("firef") {
        ("󰈹", "Firefox".to_string())
    } else if lower.contains("spot") {
        ("󰓇", "Spotify".to_string())
    } else if lower.contains("mpv") {
        ("󰐹", "MPV".to_string())
    } else if !name.is_empty() {
        ("󰎆", name.to_string())
    } else {
        ("󰎆", tr("Media", "Медиа").to_string())
    }
}

fn resolve_art(url: &str, sender: &ComponentSender<MediaModel>) -> (Option<PathBuf>, String) {
    if url.is_empty() {
        return (None, String::new());
    }

    if let Some(file_path) = url.strip_prefix("file://") {
        let p = PathBuf::from(url_decode(file_path));
        if p.exists() {
            (Some(p), url.to_string())
        } else {
            (None, url.to_string())
        }
    } else if url.starts_with("http://") || url.starts_with("https://") {
        let (cache_p, tmp_p) = get_art_cache_path(url);
        if cache_p.exists() {
            (Some(cache_p), url.to_string())
        } else {
            let target_url = url.to_string();
            let target_path = cache_p.clone();
            let s = sender.clone();
            thread::spawn(move || {
                let status = Command::new("curl")
                    .args([
                        "-fsSL",
                        "--max-time",
                        "3",
                        "-o",
                        tmp_p.to_str().unwrap_or(""),
                        &target_url,
                    ])
                    .status();
                if let Ok(st) = status
                    && st.success()
                    && tmp_p.exists()
                {
                    let _ = std::fs::rename(&tmp_p, &target_path);
                    if target_path.exists() {
                        s.input(MediaMsg::UpdateArt(target_url, Some(target_path)));
                        return;
                    }
                }
                let _ = std::fs::remove_file(&tmp_p);
                s.input(MediaMsg::UpdateArt(target_url, None));
            });
            (None, url.to_string())
        }
    } else {
        (None, url.to_string())
    }
}

pub struct MediaModel {
    info: MediaInfo,
    art_path: Option<PathBuf>,
    last_art_url: String,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    #[allow(dead_code)]
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum MediaMsg {
    PollTick,
    UpdateInfo(Option<MediaInfo>),
    UpdateArt(String, Option<PathBuf>),
    PlayPause,
    Previous,
    Next,
    Seek(f64),
    Back,
}

pub struct MediaInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for MediaModel {
    type Input = MediaMsg;
    type Output = ();
    type Init = MediaInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (390, -1),

            // 1. Header (Back, Player Badge, Spacer, Status Pill)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,
                set_valign: Align::Center,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: tr("‹ Back", "‹ Назад"),
                    set_valign: Align::Center,
                    connect_clicked => MediaMsg::Back,
                },

                #[name = "player_badge"]
                gtk4::Label {
                    add_css_class: "media-badge",
                    set_halign: Align::Start,
                    set_valign: Align::Center,
                    #[watch]
                    set_label: &{
                        let (ico, badge_text) = get_player_badge(&model.info.player_name);
                        format!("{}  {}", ico, badge_text)
                    },
                },

                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_hexpand: true,
                },

                #[name = "status_pill"]
                gtk4::Label {
                    set_valign: Align::Center,
                    #[watch]
                    set_css_classes: if model.info.is_playing {
                        &["media-status-pill"]
                    } else {
                        &["media-status-pill", "paused"]
                    },
                    #[watch]
                    set_label: if model.info.is_playing {
                        tr("󰐊  Playing", "󰐊  Играет")
                    } else {
                        tr("󰏤  Paused", "󰏤  Пауза")
                    },
                },
            },

            // 2. Track Info Card (Art, Title, Artist/Album)
            gtk4::Box {
                add_css_class: "media-card",
                set_orientation: Orientation::Horizontal,
                set_spacing: 12,

                gtk4::Box {
                    add_css_class: "media-art-box",
                    set_orientation: Orientation::Vertical,
                    set_size_request: (64, 64),
                    set_valign: Align::Center,

                    #[name = "art_pic"]
                    gtk4::Picture {
                        add_css_class: "media-art",
                        set_size_request: (64, 64),
                        set_content_fit: gtk4::ContentFit::Contain,
                        set_can_shrink: true,
                        #[watch]
                        set_visible: model.art_path.is_some(),
                        #[watch]
                        set_filename: model.art_path.as_deref(),
                    },

                    #[name = "art_fallback"]
                    gtk4::Label {
                        add_css_class: "media-art-fallback",
                        set_valign: Align::Center,
                        set_halign: Align::Center,
                        set_label: "󰎆",
                        #[watch]
                        set_visible: model.art_path.is_none(),
                    },
                },

                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 3,
                    set_valign: Align::Center,
                    set_hexpand: true,

                    gtk4::Label {
                        add_css_class: "media-title",
                        set_xalign: 0.0,
                        set_ellipsize: gtk4::pango::EllipsizeMode::End,
                        set_max_width_chars: 25,
                        #[watch]
                        set_label: if model.info.title.is_empty() {
                            tr("Nothing playing", "Ничего не воспроизводится")
                        } else {
                            &model.info.title
                        },
                    },

                    gtk4::Label {
                        add_css_class: "media-artist",
                        set_xalign: 0.0,
                        set_ellipsize: gtk4::pango::EllipsizeMode::End,
                        set_max_width_chars: 27,
                        #[watch]
                        set_label: &{
                            if model.info.title.is_empty() {
                                tr("Waiting for media stream...", "Ожидание медиапотока...").to_string()
                            } else if !model.info.album.is_empty()
                                && model.info.album != model.info.title
                                && model.info.album != model.info.artist
                            {
                                format!("{} · {}", model.info.artist, model.info.album)
                            } else if !model.info.artist.is_empty() {
                                model.info.artist.clone()
                            } else {
                                tr("Unknown artist", "Неизвестный исполнитель").to_string()
                            }
                        },
                    },
                },
            },

            // 3. Progress Bar & Time Timestamps
            gtk4::Box {
                set_orientation: Orientation::Vertical,
                set_spacing: 4,
                set_margin_start: 4,
                set_margin_end: 4,

                #[name = "progress_bar"]
                gtk4::ProgressBar {
                    add_css_class: "media-progress",
                    #[watch]
                    set_fraction: if model.info.length_us > 0 {
                        (model.info.position_us as f64 / model.info.length_us as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                },

                gtk4::Box {
                    set_orientation: Orientation::Horizontal,

                    gtk4::Label {
                        add_css_class: "media-time",
                        set_halign: Align::Start,
                        #[watch]
                        set_label: if !model.info.position_str.is_empty() {
                            &model.info.position_str
                        } else {
                            "0:00"
                        },
                    },

                    gtk4::Box {
                        set_orientation: Orientation::Horizontal,
                        set_hexpand: true,
                    },

                    gtk4::Label {
                        add_css_class: "media-time",
                        set_halign: Align::End,
                        #[watch]
                        set_label: if !model.info.length_str.is_empty() {
                            &model.info.length_str
                        } else {
                            "0:00"
                        },
                    },
                },
            },

            // 4. Playback Controls (Prev, Play/Pause, Next)
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 14,
                set_halign: Align::Center,
                set_margin_top: 2,
                set_margin_bottom: 2,

                gtk4::Button {
                    add_css_class: "media-btn",
                    set_label: "󰒮",
                    connect_clicked => MediaMsg::Previous,
                },

                #[name = "play_btn"]
                gtk4::Button {
                    add_css_class: "play-btn",
                    #[watch]
                    set_label: if model.info.is_playing { "󰏤" } else { "󰐊" },
                    connect_clicked => MediaMsg::PlayPause,
                },

                gtk4::Button {
                    add_css_class: "media-btn",
                    set_label: "󰒭",
                    connect_clicked => MediaMsg::Next,
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let initial_info = query_media_info().unwrap_or_default();
        let (art_path, last_art_url) = resolve_art(&initial_info.art_url, &sender);

        let model = MediaModel {
            info: initial_info,
            art_path,
            last_art_url,
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        // Attach gesture click for seeking on progress bar
        let s_seek = sender.clone();
        let gesture = GestureClick::new();
        gesture.connect_pressed(move |g, _, x, _| {
            if let Some(w) = g.widget() {
                let width = w.width() as f64;
                if width > 0.0 {
                    let frac = (x / width).clamp(0.0, 1.0);
                    s_seek.input(MediaMsg::Seek(frac));
                }
            }
        });
        widgets.progress_bar.add_controller(gesture);

        // Periodic 1-second refresh timer while root widget is mapped
        let root_mapped = root.clone();
        let s_timer = sender.clone();
        glib::timeout_add_local(Duration::from_millis(1000), move || {
            if !root_mapped.is_mapped() {
                return glib::ControlFlow::Break;
            }
            s_timer.input(MediaMsg::PollTick);
            glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            MediaMsg::PollTick => {
                let s = sender.clone();
                thread::spawn(move || {
                    let info = query_media_info();
                    s.input(MediaMsg::UpdateInfo(info));
                });
            }
            MediaMsg::UpdateInfo(opt_info) => {
                let info = opt_info.unwrap_or_default();
                if info.art_url != self.last_art_url {
                    let (path, url) = resolve_art(&info.art_url, &sender);
                    self.art_path = path;
                    self.last_art_url = url;
                }
                self.info = info;
            }
            MediaMsg::UpdateArt(url, path) => {
                if url == self.last_art_url {
                    self.art_path = path;
                }
            }
            MediaMsg::PlayPause => {
                self.info.is_playing = !self.info.is_playing;
                let s = sender.clone();
                thread::spawn(move || {
                    let _ = Command::new("playerctl").arg("play-pause").output();
                    thread::sleep(Duration::from_millis(120));
                    let fresh = query_media_info();
                    s.input(MediaMsg::UpdateInfo(fresh));
                });
            }
            MediaMsg::Previous => {
                let s = sender.clone();
                thread::spawn(move || {
                    let _ = Command::new("playerctl").arg("previous").output();
                    thread::sleep(Duration::from_millis(120));
                    let fresh = query_media_info();
                    s.input(MediaMsg::UpdateInfo(fresh));
                });
            }
            MediaMsg::Next => {
                let s = sender.clone();
                thread::spawn(move || {
                    let _ = Command::new("playerctl").arg("next").output();
                    thread::sleep(Duration::from_millis(120));
                    let fresh = query_media_info();
                    s.input(MediaMsg::UpdateInfo(fresh));
                });
            }
            MediaMsg::Seek(frac) => {
                let total_us = self.info.length_us;
                if total_us > 0 {
                    let target_sec = (total_us as f64 / 1_000_000.0) * frac;
                    thread::spawn(move || {
                        let _ = Command::new("playerctl")
                            .args(["position", &format!("{:.1}", target_sec)])
                            .output();
                    });
                }
            }
            MediaMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

pub fn build_media_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32, Box<dyn std::any::Any>)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = MediaInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let controller = MediaModel::builder().launch(init).detach();
    let root = controller.widget().clone();
    (root, 390, 210, Box::new(controller))
}
