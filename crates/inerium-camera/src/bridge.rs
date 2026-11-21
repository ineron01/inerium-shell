use crate::device::DeviceManager;
use crate::scrcpy::{ingest_sock_path, preview_shm_socket, session_state_file};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::fs;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;

pub const FRAME_WIDTH: i32 = 1920;
pub const FRAME_HEIGHT: i32 = 1080;
pub const FRAME_BYTES: usize = (FRAME_WIDTH as usize) * (FRAME_HEIGHT as usize) * 3 / 2; // 3,110,400 bytes for I420

struct BridgeSharedState {
    black_frame: Vec<u8>,
    last_frame: Vec<u8>,
    last_rx_at: Option<Instant>,
    active_gen: u64,
    active_readers: usize,
}

/// Build an I420 1920x1080 studio-black frame (Y=16, U/V=128)
fn build_black_i420() -> Vec<u8> {
    let y_bytes = (FRAME_WIDTH as usize) * (FRAME_HEIGHT as usize);
    let mut frame = vec![128u8; FRAME_BYTES];
    frame[..y_bytes].fill(16u8);
    frame
}

fn push_frame_to_appsrc(appsrc: &gst_app::AppSrc, frame_bytes: &[u8]) {
    let Ok(mut buffer) = gst::Buffer::with_size(frame_bytes.len()) else {
        return;
    };
    if let Some(buf_mut) = buffer.get_mut() {
        if let Ok(mut map) = buf_mut.map_writable() {
            map.as_mut_slice().copy_from_slice(frame_bytes);
        }
    }
    let _ = appsrc.push_buffer(buffer);
}

async fn resolve_v4l2_device(explicit: Option<String>) -> Result<String, String> {
    if let Some(dev) = explicit {
        if Path::new(&dev).exists() {
            return Ok(dev);
        }
    }
    for _ in 0..25 {
        if let Some(dev) = DeviceManager::find_v4l2_loopback() {
            return Ok(dev);
        }
        sleep(Duration::from_millis(200)).await;
    }
    Err("V4L2 loopback device (PhoneWebcam) not found.".to_string())
}

/// Run the persistent /dev/video0 + SHM preview bridge until SIGTERM/SIGINT or fatal sink error
pub async fn run_bridge(v4l2_opt: Option<String>) -> Result<(), String> {
    gst::init().map_err(|e| format!("Failed to init GStreamer: {}", e))?;
    let v4l2_dev = resolve_v4l2_device(v4l2_opt).await?;

    let ingest_sock = ingest_sock_path();
    let preview_sock = preview_shm_socket();

    let _ = fs::remove_file(&ingest_sock);
    let _ = fs::remove_file(&preview_sock);

    let pipe_str = format!(
        "appsrc name=src is-live=true do-timestamp=true format=time stream-type=stream block=false max-bytes=6220800 ! \
         videoconvert ! queue leaky=downstream max-size-buffers=2 ! tee name=t ! \
         queue leaky=downstream max-size-buffers=2 ! videoconvert ! video/x-raw,format=YUY2 ! v4l2sink device=\"{}\" sync=false \
         t. ! queue leaky=downstream max-size-buffers=2 ! shmsink socket-path=\"{}\" wait-for-connection=false sync=false shm-size=33554432",
        v4l2_dev, preview_sock.display()
    );

    let element = gst::parse::launch(&pipe_str)
        .map_err(|e| format!("Failed to create bridge pipeline: {}", e))?;
    let pipeline = element
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| "Failed to cast bridge element to Pipeline".to_string())?;
    let appsrc = pipeline
        .by_name("src")
        .ok_or_else(|| "Bridge appsrc element not found".to_string())?
        .dynamic_cast::<gst_app::AppSrc>()
        .map_err(|_| "Failed to cast src to AppSrc".to_string())?;

    let caps = gst::Caps::builder("video/x-raw")
        .field("format", "I420")
        .field("width", FRAME_WIDTH)
        .field("height", FRAME_HEIGHT)
        .field("framerate", gst::Fraction::new(30, 1))
        .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
        .field("interlace-mode", "progressive")
        .build();
    appsrc.set_caps(Some(&caps));

    pipeline
        .set_state(gst::State::Playing)
        .map_err(|e| format!("Failed to start bridge pipeline: {:?}", e))?;

    let listener = UnixListener::bind(&ingest_sock)
        .map_err(|e| format!("Failed to bind ingest socket {}: {}", ingest_sock.display(), e))?;

    let black = build_black_i420();
    let running = Arc::new(AtomicBool::new(true));
    let shared = Arc::new(Mutex::new(BridgeSharedState {
        black_frame: black.clone(),
        last_frame: black,
        last_rx_at: None,
        active_gen: 0,
        active_readers: 0,
    }));

    // Push initial black frame so exclusive_caps=1 activates Video Capture immediately
    {
        let initial = { shared.lock().unwrap().black_frame.clone() };
        push_frame_to_appsrc(&appsrc, &initial);
    }

    // Thread 1: Accept hot-swappable producer connections on Unix socket
    let running_accept = running.clone();
    let shared_accept = shared.clone();
    let appsrc_accept = appsrc.clone();
    std::thread::spawn(move || {
        for stream_res in listener.incoming() {
            if !running_accept.load(Ordering::Relaxed) {
                break;
            }
            let Ok(mut stream) = stream_res else {
                continue;
            };

            let my_gen = {
                let mut guard = shared_accept.lock().unwrap();
                guard.active_gen += 1;
                guard.active_readers += 1;
                guard.active_gen
            };

            let running_rd = running_accept.clone();
            let shared_rd = shared_accept.clone();
            let appsrc_rd = appsrc_accept.clone();

            std::thread::spawn(move || {
                let mut buf = vec![0u8; FRAME_BYTES];
                while running_rd.load(Ordering::Relaxed) {
                    if stream.read_exact(&mut buf).is_err() {
                        break;
                    }
                    let is_current = {
                        let mut guard = shared_rd.lock().unwrap();
                        if guard.active_gen != my_gen {
                            false
                        } else {
                            guard.last_frame.copy_from_slice(&buf);
                            guard.last_rx_at = Some(Instant::now());
                            true
                        }
                    };
                    if !is_current {
                        break;
                    }
                    push_frame_to_appsrc(&appsrc_rd, &buf);
                }

                let mut guard = shared_rd.lock().unwrap();
                guard.active_readers = guard.active_readers.saturating_sub(1);
            });
        }
    });

    // Thread 2: Keepalive ticker (~30fps) feeds last frame across swaps and reverts to black when stopped
    let running_ka = running.clone();
    let shared_ka = shared.clone();
    let appsrc_ka = appsrc.clone();
    std::thread::spawn(move || {
        let interval = Duration::from_millis(33);
        let stale_threshold = Duration::from_millis(45);
        let max_frozen_duration = Duration::from_millis(1000);
        while running_ka.load(Ordering::Relaxed) {
            std::thread::sleep(interval);
            if !running_ka.load(Ordering::Relaxed) {
                break;
            }
            let fallback_frame = {
                let mut guard = shared_ka.lock().unwrap();
                let elapsed = guard.last_rx_at.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);

                // Automatically revert to black if:
                // 1) No active readers and session file missing (normal stopped state)
                // 2) No active readers for > 1s even if session file exists (producer died unexpectedly / phone discharged)
                // 3) Readers connected but no data received for > 2.5s (producer hung/stalled)
                let should_revert_to_black = if guard.active_readers == 0 {
                    !session_state_file().exists() || elapsed >= max_frozen_duration
                } else {
                    elapsed >= Duration::from_millis(2500)
                };

                if should_revert_to_black && guard.last_rx_at.is_some() {
                    let black = guard.black_frame.clone();
                    guard.last_frame = black;
                    guard.last_rx_at = None;
                }

                let need_keepalive = match guard.last_rx_at {
                    None => true,
                    Some(t) => t.elapsed() >= stale_threshold,
                };
                if need_keepalive {
                    Some(guard.last_frame.clone())
                } else {
                    None
                }
            };
            if let Some(frame) = fallback_frame {
                push_frame_to_appsrc(&appsrc_ka, &frame);
            }
        }
    });

    // Wait for SIGTERM, SIGINT, or fatal GStreamer bus error
    let bus = pipeline.bus();
    let running_bus = running.clone();
    let (err_tx, mut err_rx) = tokio::sync::mpsc::channel::<String>(1);
    if let Some(bus) = bus {
        std::thread::spawn(move || {
            while running_bus.load(Ordering::Relaxed) {
                if let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(200)) {
                    if let gst::MessageView::Error(err) = msg.view() {
                        let _ = err_tx.try_send(format!(
                            "GStreamer bridge error ({}): {}",
                            err.error(),
                            err.debug().unwrap_or_default()
                        ));
                        break;
                    }
                }
            }
        });
    }

    let mut sigterm = signal(SignalKind::terminate())
        .map_err(|e| format!("Failed to subscribe SIGTERM: {}", e))?;
    let mut sigint = signal(SignalKind::interrupt())
        .map_err(|e| format!("Failed to subscribe SIGINT: {}", e))?;

    let mut result = Ok(());
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = sigint.recv() => {},
        Some(err_msg) = err_rx.recv() => {
            eprintln!("{}", err_msg);
            result = Err(err_msg);
        }
    }

    running.store(false, Ordering::SeqCst);
    let _ = pipeline.set_state(gst::State::Null);
    let _ = fs::remove_file(ingest_sock_path());
    let _ = fs::remove_file(preview_shm_socket());

    result
}
