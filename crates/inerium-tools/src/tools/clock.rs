//! Inerium Dragon — Native Rust Clock & Calendar Provider for Waybar
//! - Normal format: HH:MM
//! - Extended format: Day, DD Mon  HH:MM (toggled by LKM click)
//! - Tooltip: Full month calendar with Pango markup, JetBrainsMono font, and Russian names
//! - Interactive month navigation: wheel scroll shifts month, right-click resets to current
//! - Watch mode: Streams time updates, instant reaction to SIGUSR1 on toggle

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use inerium_i18n::Lang;
use inerium_tools::format::escape_json;
use inerium_tools::ipc::{PidRegistry, SelfPipe};
use inerium_tools::sys::posix::*;

fn clock_registry() -> PidRegistry {
    PidRegistry::new("clock-detailed")
}

fn seconds_registry() -> PidRegistry {
    PidRegistry::new("clock-seconds")
}

fn month_offset_path() -> PathBuf {
    let uid = posix_getuid();
    PathBuf::from(format!("/run/user/{}/inerium-clock-offset", uid))
}

fn get_month_offset() -> i32 {
    let p = month_offset_path();
    if let Ok(s) = fs::read_to_string(p) {
        s.trim().parse::<i32>().unwrap_or(0)
    } else {
        0
    }
}

fn set_month_offset(offset: i32) {
    let p = month_offset_path();
    if offset == 0 {
        let _ = fs::remove_file(p);
    } else {
        let _ = fs::write(p, offset.to_string());
    }
    PidRegistry::new("clock-detailed").notify_all(SIGUSR1);
}

fn is_detailed() -> bool {
    clock_registry().is_state_active()
}

fn toggle_detailed() {
    let reg = clock_registry();
    reg.toggle_state();
    reg.notify_all(SIGUSR1);
}

fn is_seconds_active() -> bool {
    seconds_registry().is_state_active()
}

fn toggle_seconds() {
    let reg = seconds_registry();
    reg.toggle_state();
    clock_registry().notify_all(SIGUSR1);
}

const WEEKDAY_SHORT_RU: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
const WEEKDAY_SHORT_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

const MONTH_SHORT_RU: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];
const MONTH_SHORT_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const WEEKDAY_FULL_RU: [&str; 7] = [
    "Понедельник", "Вторник", "Среда", "Четверг", "Пятница", "Суббота", "Воскресенье",
];
const WEEKDAY_FULL_EN: [&str; 7] = [
    "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
];

const MONTH_NAMES_NOMINATIVE_RU: [&str; 12] = [
    "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь",
    "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь",
];
const MONTH_NAMES_NOMINATIVE_EN: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

const MONTH_NAMES_GENITIVE_RU: [&str; 12] = [
    "января", "февраля", "марта", "апреля", "мая", "июня",
    "июля", "августа", "сентября", "октября", "ноября", "декабря",
];
const MONTH_NAMES_GENITIVE_EN: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

fn format_time() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let tm = posix_localtime_r(&now);

    let show_seconds = is_seconds_active();
    let time_str = if show_seconds {
        format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
    } else {
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    };

    if is_detailed() {
        let wday_idx = ((tm.tm_wday + 6) % 7) as usize;
        let mon_idx = (tm.tm_mon.clamp(0, 11)) as usize;
        match Lang::get() {
            Lang::Ru => {
                let w = WEEKDAY_SHORT_RU[wday_idx];
                let mut chars = w.chars();
                let capitalized = match chars.next() {
                    Some(f) => format!("{}{}", f.to_uppercase(), chars.as_str()),
                    None => String::new(),
                };
                format!(
                    "{}, {} {}  {}",
                    capitalized, tm.tm_mday, MONTH_SHORT_RU[mon_idx], time_str
                )
            }
            Lang::En => {
                format!(
                    "{}, {} {}  {}",
                    WEEKDAY_SHORT_EN[wday_idx], tm.tm_mday, MONTH_SHORT_EN[mon_idx], time_str
                )
            }
        }
    } else {
        time_str
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        0 | 2 | 4 | 6 | 7 | 9 | 11 => 31,
        3 | 5 | 8 | 10 => 30,
        1 => if is_leap_year(year) { 29 } else { 28 },
        _ => 30,
    }
}

// Tomohiko Sakamoto's algorithm for day of week (0 = Sunday, 1 = Monday, ..., 6 = Saturday)
fn day_of_week(y: i32, m: i32, d: i32) -> i32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = y;
    if m < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + t[(m - 1) as usize] + d) % 7
}

fn format_tooltip() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let tm = posix_localtime_r(&now);

    let real_year = tm.tm_year + 1900;
    let real_month = tm.tm_mon.clamp(0, 11) as usize;
    let today = tm.tm_mday;
    let rus_wday = ((tm.tm_wday + 6) % 7) as usize;

    let offset = get_month_offset();
    let total_months = (real_year as i64) * 12 + (real_month as i64) + (offset as i64);
    let display_year = total_months.div_euclid(12) as i32;
    let display_month = total_months.rem_euclid(12) as usize;

    let total_days = days_in_month(display_year, display_month as i32);
    // Calculate weekday of 1st day of target month (0 = Monday, ..., 6 = Sunday)
    let dow1 = day_of_week(display_year, (display_month + 1) as i32, 1);
    let first_wday = ((dow1 + 6) % 7) as usize;

    let lang = Lang::get();
    let (wday_full, mon_gen, mon_nom) = match lang {
        Lang::Ru => (WEEKDAY_FULL_RU, MONTH_NAMES_GENITIVE_RU, MONTH_NAMES_NOMINATIVE_RU),
        Lang::En => (WEEKDAY_FULL_EN, MONTH_NAMES_GENITIVE_EN, MONTH_NAMES_NOMINATIVE_EN),
    };

    let header = match lang {
        Lang::Ru => format!("{}, {} {} {}", wday_full[rus_wday], today, mon_gen[real_month], real_year),
        Lang::En => format!("{}, {} {} {}", wday_full[rus_wday], mon_nom[real_month], today, real_year),
    };
    let month_year = format!("‹ {} {} ›", mon_nom[display_month], display_year);

    let col_w = 5usize;
    let total_w = col_w * 7; // 35 chars width — compact and balanced

    let head_pad = total_w.saturating_sub(header.chars().count()) / 2;
    let head_spaces = " ".repeat(head_pad);

    let pad_m = total_w.saturating_sub(month_year.chars().count()) / 2;
    let pad_spaces = " ".repeat(pad_m);

    let mut output = String::with_capacity(1024);
    output.push_str("<span font_desc='JetBrainsMono Nerd Font 10'>");

    // Header: full current date
    output.push_str(&format!(
        "<span weight='bold' color='#7fb4ca'>{}{}\n</span>\n",
        head_spaces, header
    ));

    // Month and Year navigator (with equal spacing above and below)
    output.push_str(&format!(
        "<span weight='bold' color='#8ba4b0'>{}{}\n</span>\n",
        pad_spaces, month_year
    ));

    // Weekdays with weekend highlight (5 chars per column)
    match lang {
        Lang::Ru => {
            output.push_str(" Пн   Вт   Ср   Чт   Пт  <span color='#c4746e'> Сб</span>  <span color='#c4746e'> Вс</span>  \n");
        }
        Lang::En => {
            output.push_str(" Mo   Tu   We   Th   Fr  <span color='#c4746e'> Sa</span>  <span color='#c4746e'> Su</span>  \n");
        }
    }

    // Leading empty days
    output.push_str(&" ".repeat(col_w * first_wday));

    let is_current_month = display_year == real_year && display_month == real_month;
    let mut cur_col = first_wday;
    for d in 1..=total_days {
        if is_current_month && d == today {
            output.push_str(&format!("<span weight='bold' color='#7fb4ca'>[{:2}]</span> ", d));
        } else if cur_col == 5 || cur_col == 6 {
            output.push_str(&format!(" <span color='#c4746e'>{:2}</span>  ", d));
        } else {
            output.push_str(&format!(" {:2}  ", d));
        }
        cur_col += 1;
        if cur_col == 7 {
            output.push('\n');
            cur_col = 0;
        }
    }
    if cur_col != 0 {
        output.push('\n');
    }

    let (f1, f2) = match lang {
        Lang::Ru => ("ЛКМ: Дата · СКМ: Секунды", "ПКМ: Сброс · Колесо: Месяцы"),
        Lang::En => ("LMB: Date · MMB: Seconds", "RMB: Reset · Wheel: Months"),
    };
    let pad_f1 = " ".repeat(total_w.saturating_sub(f1.chars().count()) / 2);
    let pad_f2 = " ".repeat(total_w.saturating_sub(f2.chars().count()) / 2);

    output.push_str(&format!(
        "\n<span color='#727169'>{}{}\n{}{}</span>",
        pad_f1, f1, pad_f2, f2
    ));
    output.push_str("</span>");
    output
}

fn format_json(tooltip: &str) -> String {
    let text = format_time();
    format!(
        "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":\"clock\"}}",
        escape_json(&text),
        escape_json(tooltip)
    )
}

fn run_watch(waybar: bool) {
    let reg = clock_registry();
    let _pid_guard = reg.register();
    let self_pipe = match SelfPipe::new() {
        Ok(sp) => sp,
        Err(e) => {
            eprintln!("Failed to create self pipe: {e}");
            return;
        }
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut last_output = String::new();

    let offset_path = month_offset_path();

    let mut cached_tooltip = String::new();
    let mut last_tooltip_key = (-1, -1, Lang::Ru);

    loop {
        // Auto-reset month offset after 15 seconds of inactivity
        if offset_path.exists()
            && let Ok(meta) = fs::metadata(&offset_path)
            && let Ok(modified) = meta.modified()
            && let Ok(elapsed) = modified.elapsed()
            && elapsed >= std::time::Duration::from_secs(15) {
                let _ = fs::remove_file(&offset_path);
            }

        let now_dur = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let now_secs = now_dur.as_secs() as i64;
        let subsec_ms = now_dur.subsec_millis() as i32;

        let tm = posix_localtime_r(&now_secs);

        let current = if waybar {
            let offset = get_month_offset();
            let lang = Lang::get();
            if last_tooltip_key != (tm.tm_mday, offset, lang) || cached_tooltip.is_empty() {
                cached_tooltip = format_tooltip();
                last_tooltip_key = (tm.tm_mday, offset, lang);
            }
            format_json(&cached_tooltip)
        } else {
            format_time()
        };

        if current != last_output {
            let _ = writeln!(handle, "{}", current);
            let _ = handle.flush();
            last_output = current;
        }

        let timeout_ms = if is_seconds_active() {
            // Align sleep to the exact start of the next second (+ 3ms guard margin)
            (1000 - subsec_ms + 3).clamp(1, 1000)
        } else if offset_path.exists() {
            // Month offset active: poll every 1s for 15s auto-reset
            1000
        } else {
            // Align sleep to the next minute boundary (+ 5ms guard margin)
            let sec = (now_secs % 60) as i32;
            ((60 - sec) * 1000 - subsec_ms + 5).clamp(1, 60_000)
        };

        if self_pipe.wait_event(timeout_ms) {
            Lang::reinit();
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--toggle" => {
                toggle_detailed();
                println!("{}", format_time());
                return;
            }
            "--toggle-seconds" | "--toggle-sec" => {
                toggle_seconds();
                println!("{}", format_time());
                return;
            }
            "--tooltip" => {
                println!("{}", format_tooltip());
                return;
            }
            "--next-month" => {
                set_month_offset(get_month_offset() + 1);
                return;
            }
            "--prev-month" => {
                set_month_offset(get_month_offset() - 1);
                return;
            }
            "--reset-month" => {
                set_month_offset(0);
                return;
            }
            "--waybar" => {
                run_watch(true);
                return;
            }
            "--watch" => {
                let waybar = args.iter().any(|a| a == "--json" || a == "--waybar");
                run_watch(waybar);
                return;
            }
            _ => {}
        }
    }
    println!("{}", format_time());
}
