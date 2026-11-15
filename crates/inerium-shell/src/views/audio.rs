use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation};
use inerium_i18n::tr;
use relm4::prelude::*;

use crate::hw;

pub struct AudioViewModel {
    out_volume: u32,
    out_muted: bool,
    mic_volume: u32,
    mic_muted: bool,
    sink_rows: Vec<(u32, Button, Label)>,
    source_rows: Vec<(u32, Button, Label)>,
    nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[derive(Debug)]
pub enum AudioMsg {
    SetOutputVolume(u32),
    ToggleOutputMute,
    SetMicVolume(u32),
    ToggleMicMute,
    SelectSink(u32),
    SelectSource(u32),
    OpenMixer,
    Back,
}

pub struct AudioInit {
    pub nav_cb: std::boxed::Box<dyn Fn(&str) + 'static>,
    pub close_cb: std::boxed::Box<dyn Fn() + 'static>,
}

#[relm4::component(pub)]
impl SimpleComponent for AudioViewModel {
    type Input = AudioMsg;
    type Output = ();
    type Init = AudioInit;

    view! {
        #[root]
        gtk4::Box {
            add_css_class: "popup-container",
            set_orientation: Orientation::Vertical,
            set_spacing: 10,
            set_size_request: (380, -1),

            // Header
            gtk4::Box {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,

                gtk4::Button {
                    add_css_class: "back-btn",
                    set_label: &tr("‹ Back", "‹ Назад"),
                    connect_clicked => AudioMsg::Back,
                },

                gtk4::Label {
                    add_css_class: "cc-host",
                    set_label: &tr("Audio Devices", "Аудиоустройства"),
                    set_hexpand: true,
                    set_xalign: 0.0,
                },

                gtk4::Button {
                    add_css_class: "cc-chip",
                    set_label: &tr("󰕾 Mixer", "󰕾 Микшер"),
                    connect_clicked => AudioMsg::OpenMixer,
                },
            },

            // Volume Sliders Card
            gtk4::Box {
                add_css_class: "cc-card",
                set_orientation: Orientation::Vertical,
                set_spacing: 8,

                // Output Row
                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 8,

                    #[name = "out_mute_btn"]
                    gtk4::Button {
                        add_css_class: "cc-slider-ico",
                        #[watch]
                        set_css_classes: if model.out_muted { &["cc-slider-ico", "muted"] } else { &["cc-slider-ico"] },
                        #[watch]
                        set_label: if model.out_muted { "󰝟" } else { "󰕾" },
                        set_tooltip_text: Some(tr("Mute Output", "Заглушить вывод")),
                        connect_clicked => AudioMsg::ToggleOutputMute,
                    },

                    #[name = "out_scale"]
                    gtk4::Scale {
                        add_css_class: "cc-scale",
                        set_orientation: Orientation::Horizontal,
                        set_range: (0.0, 150.0),
                        set_hexpand: true,
                        set_value: model.out_volume as f64,
                        connect_value_changed[sender] => move |sc| {
                            sender.input(AudioMsg::SetOutputVolume(sc.value() as u32));
                        },
                    },

                    #[name = "out_pct_lbl"]
                    gtk4::Label {
                        add_css_class: "cc-pct",
                        #[watch]
                        set_label: &format!("{}%", model.out_volume),
                    },
                },

                // Input Row
                gtk4::Box {
                    set_orientation: Orientation::Horizontal,
                    set_spacing: 8,

                    #[name = "mic_mute_btn"]
                    gtk4::Button {
                        add_css_class: "cc-slider-ico",
                        #[watch]
                        set_css_classes: if model.mic_muted { &["cc-slider-ico", "muted"] } else { &["cc-slider-ico"] },
                        #[watch]
                        set_label: if model.mic_muted { "󰍭" } else { "󰍬" },
                        set_tooltip_text: Some(tr("Mute Microphone", "Заглушить микрофон")),
                        connect_clicked => AudioMsg::ToggleMicMute,
                    },

                    #[name = "mic_scale"]
                    gtk4::Scale {
                        add_css_class: "cc-scale",
                        set_orientation: Orientation::Horizontal,
                        set_range: (0.0, 150.0),
                        set_hexpand: true,
                        set_value: model.mic_volume as f64,
                        connect_value_changed[sender] => move |sc| {
                            sender.input(AudioMsg::SetMicVolume(sc.value() as u32));
                        },
                    },

                    #[name = "mic_pct_lbl"]
                    gtk4::Label {
                        add_css_class: "cc-pct",
                        #[watch]
                        set_label: &format!("{}%", model.mic_volume),
                    },
                },
            },

            // Devices List
            gtk4::ScrolledWindow {
                add_css_class: "list-scroll",
                set_policy: (gtk4::PolicyType::Never, gtk4::PolicyType::Automatic),
                set_propagate_natural_height: true,
                set_min_content_height: 140,
                set_max_content_height: 260,
                set_vexpand: true,

                #[name = "list_box"]
                gtk4::Box {
                    set_orientation: Orientation::Vertical,
                    set_spacing: 4,
                },
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let out_volume = hw::get_volume();
        let out_muted = hw::is_volume_muted();
        let mic_volume = hw::get_mic_volume();
        let mic_muted = hw::is_mic_muted();

        let mut sink_rows = Vec::new();
        let mut source_rows = Vec::new();

        let model = AudioViewModel {
            out_volume,
            out_muted,
            mic_volume,
            mic_muted,
            sink_rows: Vec::new(),
            source_rows: Vec::new(),
            nav_cb: init.nav_cb,
            close_cb: init.close_cb,
        };

        let widgets = view_output!();

        // Section: Output Devices
        let out_title = Label::new(Some(tr("OUTPUT DEVICES", "УСТРОЙСТВА ВЫВОДА")));
        out_title.add_css_class("cc-label");
        out_title.set_xalign(0.0);
        widgets.list_box.append(&out_title);

        let sinks = hw::list_audio_sinks();
        for sink in sinks {
            let row = Button::new();
            row.add_css_class("card-item");
            if sink.is_default {
                row.add_css_class("active");
            }

            let inner = gtk4::Box::new(Orientation::Horizontal, 8);
            let is_headphone = sink.name.to_lowercase().contains("headphone")
                || sink.name.to_lowercase().contains("buds");
            let ico = Label::new(Some(if is_headphone { "󰋋" } else { "󰕾" }));
            ico.add_css_class("card-icon");
            inner.append(&ico);

            let txt = gtk4::Box::new(Orientation::Vertical, 1);
            txt.set_hexpand(true);
            let name_lbl = Label::new(Some(&sink.name));
            name_lbl.add_css_class("card-title");
            name_lbl.set_xalign(0.0);
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            txt.append(&name_lbl);
            inner.append(&txt);

            let badge = Label::new(Some(if sink.is_default {
                tr("󰄬 Active", "󰄬 Активен")
            } else {
                ""
            }));
            badge.add_css_class("card-badge");
            inner.append(&badge);

            row.set_child(Some(&inner));

            let s_id = sink.id;
            let s_sender = sender.clone();
            row.connect_clicked(move |_| {
                s_sender.input(AudioMsg::SelectSink(s_id));
            });

            widgets.list_box.append(&row);
            sink_rows.push((s_id, row, badge));
        }

        // Section: Input Devices
        let in_title = Label::new(Some(tr("INPUT DEVICES", "УСТРОЙСТВА ВВОДА")));
        in_title.add_css_class("cc-label");
        in_title.set_xalign(0.0);
        in_title.set_margin_top(8);
        widgets.list_box.append(&in_title);

        let sources = hw::list_audio_sources();
        for source in sources {
            let row = Button::new();
            row.add_css_class("card-item");
            if source.is_default {
                row.add_css_class("active");
            }

            let inner = gtk4::Box::new(Orientation::Horizontal, 8);
            let ico = Label::new(Some("󰍬"));
            ico.add_css_class("card-icon");
            inner.append(&ico);

            let txt = gtk4::Box::new(Orientation::Vertical, 1);
            txt.set_hexpand(true);
            let name_lbl = Label::new(Some(&source.name));
            name_lbl.add_css_class("card-title");
            name_lbl.set_xalign(0.0);
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            txt.append(&name_lbl);
            inner.append(&txt);

            let badge = Label::new(Some(if source.is_default {
                tr("󰄬 Active", "󰄬 Активен")
            } else {
                ""
            }));
            badge.add_css_class("card-badge");
            inner.append(&badge);

            row.set_child(Some(&inner));

            let src_id = source.id;
            let src_sender = sender.clone();
            row.connect_clicked(move |_| {
                src_sender.input(AudioMsg::SelectSource(src_id));
            });

            widgets.list_box.append(&row);
            source_rows.push((src_id, row, badge));
        }

        let mut model = model;
        model.sink_rows = sink_rows;
        model.source_rows = source_rows;

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AudioMsg::SetOutputVolume(val) => {
                self.out_volume = val;
                hw::set_volume(val);
            }
            AudioMsg::ToggleOutputMute => {
                hw::toggle_volume_mute();
                self.out_muted = hw::is_volume_muted();
            }
            AudioMsg::SetMicVolume(val) => {
                self.mic_volume = val;
                hw::set_mic_volume(val);
            }
            AudioMsg::ToggleMicMute => {
                hw::toggle_mic_mute();
                self.mic_muted = hw::is_mic_muted();
            }
            AudioMsg::SelectSink(id) => {
                hw::set_default_sink(id);
                for (s_id, row, badge) in &self.sink_rows {
                    if *s_id == id {
                        row.add_css_class("active");
                        badge.set_label(tr("󰄬 Active", "󰄬 Активен"));
                    } else {
                        row.remove_css_class("active");
                        badge.set_label("");
                    }
                }
            }
            AudioMsg::SelectSource(id) => {
                hw::set_default_source(id);
                for (s_id, row, badge) in &self.source_rows {
                    if *s_id == id {
                        row.add_css_class("active");
                        badge.set_label(tr("󰄬 Active", "󰄬 Активен"));
                    } else {
                        row.remove_css_class("active");
                        badge.set_label("");
                    }
                }
            }
            AudioMsg::OpenMixer => {
                (self.close_cb)();
                let _ = std::process::Command::new("uwsm")
                    .args(["app", "--", "pavucontrol"])
                    .spawn()
                    .or_else(|_| std::process::Command::new("pavucontrol").spawn());
            }
            AudioMsg::Back => {
                (self.nav_cb)("control");
            }
        }
    }
}

pub fn build_audio_view<FNav, FClose>(
    nav_cb: FNav,
    close_cb: FClose,
) -> (gtk4::Box, i32, i32)
where
    FNav: Fn(&str) + 'static + Clone,
    FClose: Fn() + 'static + Clone,
{
    let init = AudioInit {
        nav_cb: std::boxed::Box::new(nav_cb),
        close_cb: std::boxed::Box::new(close_cb),
    };

    let mut controller = AudioViewModel::builder().launch(init).detach();
    controller.detach_runtime();
    let root = controller.widget().clone();
    (root, 380, 420)
}
