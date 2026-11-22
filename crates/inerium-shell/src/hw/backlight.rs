use std::process::Command;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;

pub use inerium_hw::backlight::has_external_monitor;

// -------------------------------------------------------------
// DDC/CI (External Monitor via ddcutil)
// -------------------------------------------------------------
enum DdcCmd {
    Set(u32),
}

static DDC_TX: OnceLock<Sender<DdcCmd>> = OnceLock::new();
static DDC_CACHE: Mutex<u32> = Mutex::new(50);
static DDC_BUS: OnceLock<String> = OnceLock::new();

fn get_ddc_bus() -> &'static str {
    DDC_BUS.get_or_init(|| {
        if let Ok(output) = Command::new("ddcutil").args(["detect", "--terse"]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                if let Some(pos) = line.find("/dev/i2c-") {
                    let num: String = line[pos + 9..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if !num.is_empty() {
                        return num;
                    }
                }
            }
        }
        "3".to_string()
    })
}


fn init_ddc_worker() -> Sender<DdcCmd> {
    let (tx, rx) = channel::<DdcCmd>();
    thread::spawn(move || {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                DdcCmd::Set(mut val) => {
                    while let Ok(DdcCmd::Set(newer)) = rx.try_recv() {
                        val = newer;
                    }
                    let bus = get_ddc_bus();
                    let _ = Command::new("ddcutil")
                        .args(["setvcp", "10", &val.to_string(), "--bus", bus])
                        .output();
                    if let Ok(mut g) = DDC_CACHE.lock() {
                        *g = val;
                    }
                }
            }
        }
    });
    tx
}

pub fn get_cached_ddc_brightness() -> u32 {
    DDC_CACHE.lock().map(|g| *g).unwrap_or(50)
}

pub fn refresh_ddc_brightness_async() {
    if !has_external_monitor() {
        return;
    }
    thread::spawn(move || {
        let bus = get_ddc_bus();
        if let Ok(output) = Command::new("ddcutil")
            .args(["getvcp", "10", "--bus", bus, "--brief"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() >= 4
                && let Ok(v) = parts[3].parse::<u32>() {
                    if let Ok(mut g) = DDC_CACHE.lock() {
                        *g = v;
                    }
                    glib::MainContext::default().invoke(move || {
                        crate::views::control_center::on_ddc_updated(v);
                    });
                }
        }
    });
}

#[allow(dead_code)]
pub fn get_ddc_brightness() -> u32 {
    get_cached_ddc_brightness()
}

pub fn set_ddc_brightness_async(val: u32) {
    if !has_external_monitor() {
        return;
    }
    let tx = DDC_TX.get_or_init(init_ddc_worker);
    let _ = tx.send(DdcCmd::Set(val.clamp(0, 100)));
}
