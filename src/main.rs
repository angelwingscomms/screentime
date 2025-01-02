use std::fs::File;
use std::io::Error;
use std::time::{Duration, Instant};
use chrono::Local;
use csv::Writer;
use std::thread::sleep;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, GetPropertyReply, ConnectionExt};
use x11rb::rust_connection::RustConnection;

#[derive(Debug)]
struct AppUsage {
    window_title: String,
    start_time: String,
    duration_secs: u64,
}

/// Writes the app usage data to CSV.
fn write_to_csv(usage: &AppUsage) -> Result<(), Error> {
    let file_path = "screen_time_log.csv";
    let mut wtr = Writer::from_writer(File::options().append(true).create(true).open(file_path)?);
    wtr.write_record(&[&usage.window_title, &usage.start_time, &usage.duration_secs.to_string()])?;
    wtr.flush()?;
    Ok(())
}

/// Get active window title on Linux (X11-based).
fn get_active_window_title() -> Option<String> {
    let (conn, screen_num) = RustConnection::connect(None).ok()?;
    let root = conn.setup().roots[screen_num].root;

    // Get the active window atom
    let active_window_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW").ok()?.reply().ok()?.atom;

    // Get the UTF8 string atom for _NET_WM_NAME
    let utf8_string_atom = conn.intern_atom(false, b"UTF8_STRING").ok()?.reply().ok()?.atom;
    let net_wm_name_atom = conn.intern_atom(false, b"_NET_WM_NAME").ok()?.reply().ok()?.atom;

    // Get the active window
    let active_window = conn.get_property(false, root, active_window_atom, AtomEnum::WINDOW, 0, 1)
        .ok()?.reply().ok()?.value32()?.next()?;

    // Try to get _NET_WM_NAME (modern window managers)
    if let Ok(prop) = conn.get_property(false, active_window, net_wm_name_atom, utf8_string_atom, 0, 1024).ok()?.reply() {
        if let Ok(title) = String::from_utf8(prop.value) {
            return Some(title);
        }
    }

    // Fallback: Try to get WM_NAME (older window managers)
    let wm_name_atom = conn.intern_atom(false, b"WM_NAME").ok()?.reply().ok()?.atom;
    if let Ok(prop) = conn.get_property(false, active_window, wm_name_atom, AtomEnum::STRING, 0, 1024).ok()?.reply() {
        if let Ok(title) = String::from_utf8(prop.value) {
            return Some(title);
        }
    }

    None
}


fn main() -> Result<(), Error> {
    let mut previous_title = String::new();
    let mut start_time = Instant::now();

    loop {
        // Get the current active window title
        let current_title = get_active_window_title().unwrap_or("Unknown".to_string());

        // Check if the window has changed
        if current_title != previous_title {
            let duration = start_time.elapsed().as_secs();

            // Log the previous window if it exists
            if !previous_title.is_empty() {
                let usage = AppUsage {
                    window_title: previous_title.clone(),
                    start_time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    duration_secs: duration,
                };

                // Write data to CSV and handle any errors
                if let Err(e) = write_to_csv(&usage) {
                    eprintln!("Error writing to CSV: {}", e);
                }
            }

            // Update to the new active window and reset timer
            previous_title = current_title;
            start_time = Instant::now();
        }

        // Check the active window every second
        sleep(Duration::from_secs(1));
    }
}