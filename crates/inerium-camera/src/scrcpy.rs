use crate::device::{BuiltinStatus, DeviceManager};
use crate::types::{CameraSessionInfo, CameraSource, Orientation};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::fs::{self, File, OpenOptions};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::time::sleep;
use inerium_i18n::tr;

pub fn runtime_dir() -> PathBuf {
    let rt = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", nix::unistd::getuid().as_raw()));
    let dir = PathBuf::from(rt).join("inerium");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn session_state_file() -> PathBuf {
    runtime_dir().join("camera.json")
}

pub fn preview_shm_socket() -> PathBuf {
    runtime_dir().join("cam-preview.sock")
}

pub fn ingest_sock_path() -> PathBuf {
    runtime_dir().join("cam-ingest.sock")
}

pub fn stream_fifo_path() -> PathBuf {
    runtime_dir().join("camera-stream.fifo")
}

pub fn bridge_log_file() -> PathBuf {
    runtime_dir().join("camera-bridge.log")
}

pub fn gst_log_file() -> PathBuf {
    runtime_dir().join("camera-gst.log")
}

pub fn scrcpy_log_file() -> PathBuf {
    runtime_dir().join("camera-scrcpy.log")
}

pub struct StreamingManager;

impl StreamingManager {
    pub fn is_pid_alive(pid: u32) -> bool {
        pid > 0 && kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    fn find_running_bridge_pid() -> Option<u32> {
        let output = std::process::Command::new("pgrep")
            .args(["-f", "inerium-camera _bridge"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .find(|&pid| pid != std::process::id() && Self::is_pid_alive(pid))
    }

    /// Load active producer session info (idle persistent _bridge returns is_streaming=false)
    pub fn get_current_session() -> CameraSessionInfo {
        if let Ok(data) = fs::read_to_string(session_state_file()) {
            if let Ok(info) = serde_json::from_str::<CameraSessionInfo>(&data) {
                if info.is_streaming && info.pid.is_some_and(Self::is_pid_alive) {
                    return info;
                }
            }
            Self::clear_session();
        }

        CameraSessionInfo::default()
    }

    pub fn save_session(info: &CameraSessionInfo) {
        if let Ok(json) = serde_json::to_string_pretty(info) {
            let _ = fs::write(session_state_file(), json);
        }
    }

    pub fn clear_session() {
        let _ = fs::remove_file(session_state_file());
    }

    /// Terminate an entire process group gracefully (SIGTERM -> wait -> SIGKILL)
    async fn kill_pid_gracefully(pid: u32) {
        if pid == 0 {
            return;
        }
        let pgid = Pid::from_raw(-(pid as i32));
        let direct = Pid::from_raw(pid as i32);

        if kill(pgid, Signal::SIGTERM).is_err() {
            let _ = kill(direct, Signal::SIGTERM);
        }

        for _ in 0..10 {
            sleep(Duration::from_millis(80)).await;
            if !Self::is_pid_alive(pid) {
                return;
            }
        }

        if Self::is_pid_alive(pid) {
            if kill(pgid, Signal::SIGKILL).is_err() {
                let _ = kill(direct, Signal::SIGKILL);
            }
            sleep(Duration::from_millis(40)).await;
        }
    }

    async fn kill_by_pattern(pattern: &str) {
        let _ = tokio::process::Command::new("pkill")
            .args(["-TERM", "-f", pattern])
            .output()
            .await;

        for _ in 0..8 {
            sleep(Duration::from_millis(60)).await;
            if let Ok(out) = tokio::process::Command::new("pgrep")
                .args(["-f", pattern])
                .output()
                .await
            {
                if out.stdout.is_empty() {
                    return;
                }
            }
        }

        let _ = tokio::process::Command::new("pkill")
            .args(["-KILL", "-f", pattern])
            .output()
            .await;
    }

    fn cleanup_producer_files() {
        for path in [stream_fifo_path(), gst_log_file(), scrcpy_log_file()] {
            let _ = fs::remove_file(path);
        }
    }

    fn open_log_pair(path: &Path) -> (Stdio, Stdio) {
        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
        {
            Ok(file) => match file.try_clone() {
                Ok(cloned) => (Stdio::from(file), Stdio::from(cloned)),
                Err(_) => (Stdio::null(), Stdio::from(file)),
            },
            Err(_) => (Stdio::null(), Stdio::null()),
        }
    }

    fn read_log_tail(path: &Path) -> String {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| {
                let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
                if lines.is_empty() {
                    None
                } else {
                    Some(lines[lines.len().saturating_sub(4)..].join(" | "))
                }
            })
            .unwrap_or_default()
    }

    /// Ensure the persistent _bridge daemon is running and owns /dev/video0 + preview SHM
    pub async fn ensure_bridge_ready() -> Result<u32, String> {
        let v4l2_dev = DeviceManager::find_v4l2_loopback()
            .ok_or_else(|| tr("Virtual device PhoneWebcam (/dev/video0) not found.", "Виртуальное устройство PhoneWebcam (/dev/video0) не найдено.").to_string())?;
        Self::ensure_bridge_running(&v4l2_dev).await
    }

    async fn ensure_bridge_running(v4l2_dev: &str) -> Result<u32, String> {
        if let Some(bpid) = Self::find_running_bridge_pid() {
            if ingest_sock_path().exists() && preview_shm_socket().exists() {
                return Ok(bpid);
            }
        }

        // Prefer starting the systemd --user persistent bridge unit when available
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", "start", "inerium-camera-bridge.service"])
            .output()
            .await;

        for _ in 0..30 {
            if let Some(bpid) = Self::find_running_bridge_pid() {
                if ingest_sock_path().exists() && preview_shm_socket().exists() {
                    return Ok(bpid);
                }
            }
            sleep(Duration::from_millis(50)).await;
        }

        Self::kill_by_pattern("inerium-camera _bridge").await;
        let _ = fs::remove_file(ingest_sock_path());
        let _ = fs::remove_file(preview_shm_socket());

        let exe = std::env::current_exe()
            .map_err(|e| format!("{}: {}", tr("Failed to get binary path", "Не удалось определить путь бинарника"), e))?;
        let (out_stdio, err_stdio) = Self::open_log_pair(&bridge_log_file());

        let mut cmd = tokio::process::Command::new(exe);
        cmd.args(["_bridge", "--v4l2", v4l2_dev])
            .stdin(Stdio::null())
            .stdout(out_stdio)
            .stderr(err_stdio)
            .process_group(0);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("{}: {}", tr("Failed to spawn _bridge process", "Не удалось запустить процесс-мост _bridge"), e))?;
        let bpid = child.id().unwrap_or(0);
        tokio::spawn(async move {
            let _ = child.wait().await;
        });

        for _ in 0..80 {
            sleep(Duration::from_millis(50)).await;
            if !Self::is_pid_alive(bpid) {
                let err = Self::read_log_tail(&bridge_log_file());
                return Err(if err.is_empty() {
                    tr("Bridge process _bridge exited immediately after start.", "Процесс-мост _bridge завершился сразу после запуска.").to_string()
                } else {
                    format!("{}: {}", tr("Bridge error", "Ошибка _bridge"), err)
                });
            }
            if ingest_sock_path().exists() && preview_shm_socket().exists() {
                return Ok(bpid);
            }
        }

        Self::kill_pid_gracefully(bpid).await;
        Err(tr("Timeout waiting for _bridge initialization.", "Превышено время ожидания инициализации _bridge.").to_string())
    }

    /// Build GStreamer producer tail writing exact 1:1 PAR 1920x1080@30fps I420 frames into stdout (fd=1)
    fn append_ingest_producer_args(args: &mut Vec<String>) {
        let tail = [
            "!",
            "videoflip",
            "video-direction=auto",
            "!",
            "videoconvert",
            "!",
            "videorate",
            "skip-to-first=true",
            "!",
            "videoscale",
            "add-borders=true",
            "!",
            "video/x-raw,format=I420,width=1920,height=1080,framerate=30/1,pixel-aspect-ratio=1/1",
            "!",
            "fdsink",
            "fd=1",
            "sync=false",
        ];
        for item in tail {
            args.push(item.to_string());
        }
    }

    async fn spawn_gst_producer(args: Vec<String>) -> Result<u32, String> {
        let stream = UnixStream::connect(ingest_sock_path())
            .map_err(|e| format!("{}: {}", tr("Failed to connect to bridge socket", "Не удалось подключиться к сокету моста"), e))?;
        let stdout_fd: Stdio = OwnedFd::from(stream).into();
        let stderr_stdio = File::create(gst_log_file())
            .map(Stdio::from)
            .unwrap_or_else(|_| Stdio::null());

        let mut cmd = tokio::process::Command::new("gst-launch-1.0");
        cmd.arg("-q")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(stdout_fd)
            .stderr(stderr_stdio)
            .process_group(0);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("{}: {}", tr("Failed to launch gst-launch-1.0", "Не удалось запустить gst-launch-1.0"), e))?;
        let pid = child.id().unwrap_or(0);
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(pid)
    }

    /// Stop only the active input producers (gst-launch / scrcpy) while preserving _bridge
    async fn stop_producers_only(session: &CameraSessionInfo) {
        let mut pids: Vec<u32> = Vec::new();
        for &aux in &session.aux_pids {
            if aux > 0 && !pids.contains(&aux) {
                pids.push(aux);
            }
        }
        if let Some(pid) = session.pid {
            if pid > 0 && !pids.contains(&pid) {
                pids.push(pid);
            }
        }

        for pid in pids {
            Self::kill_pid_gracefully(pid).await;
        }

        Self::cleanup_producer_files();
    }

    async fn wait_for_builtin_producer(gst_pid: u32) -> Result<(), String> {
        for _ in 0..10 {
            sleep(Duration::from_millis(80)).await;
            if !Self::is_pid_alive(gst_pid) {
                let full_log = fs::read_to_string(gst_log_file()).unwrap_or_default();
                let lower = full_log.to_lowercase();
                if lower.contains("busy")
                    || lower.contains("занято")
                    || lower.contains("resource")
                {
                    return Err(
                        tr(
                            "Built-in camera is already in use by browser or Discord. Switch camera in Discord to PhoneWebcam and try again.",
                            "Встроенная камера уже занята браузером/Discord. Переключите камеру в Discord на PhoneWebcam и повторите."
                        ).to_string(),
                    );
                }
                let err = Self::read_log_tail(&gst_log_file());
                return Err(if err.is_empty() {
                    tr("Built-in camera stream exited with an error.", "Встроенная камера завершила поток с ошибкой.").to_string()
                } else {
                    format!("{}: {}", tr("Built-in camera error", "Ошибка встроенной камеры"), err)
                });
            }
        }
        Ok(())
    }

    /// Total bytes written by a process (frames pushed into the bridge socket)
    fn bytes_written(pid: u32) -> u64 {
        fs::read_to_string(format!("/proc/{}/io", pid))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("wchar:").and_then(|v| v.trim().parse().ok()))
            })
            .unwrap_or(0)
    }

    async fn wait_for_external_producer(gst_pid: u32, scrcpy_pid: u32) -> Result<(), String> {
        // scrcpy INFO lines are block-buffered in the log, so judge by real frames reaching the bridge
        let min_bytes = (crate::bridge::FRAME_BYTES as u64) * 3;
        for _ in 0..240 {
            sleep(Duration::from_millis(50)).await;
            let log = fs::read_to_string(scrcpy_log_file()).unwrap_or_default();
            let camera_failed = log.contains("Camera capture failed") || log.contains("Camera disconnected");
            if camera_failed || !Self::is_pid_alive(gst_pid) || !Self::is_pid_alive(scrcpy_pid) {
                let scrcpy_err = Self::read_log_tail(&scrcpy_log_file());
                let gst_err = Self::read_log_tail(&gst_log_file());
                let detail = [scrcpy_err, gst_err]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ; ");
                return Err(if detail.is_empty() {
                    tr("scrcpy/GStreamer process exited with error during start.", "Процесс scrcpy/GStreamer завершился с ошибкой при запуске.").to_string()
                } else {
                    format!("{}: {}", tr("Streaming error", "Ошибка трансляции"), detail)
                });
            }

            if Self::bytes_written(gst_pid) >= min_bytes {
                return Ok(());
            }
        }
        Err(tr("Phone camera did not start sending video frames within 12 seconds.", "Камера телефона не начала передавать видео за 12 секунд.").to_string())
    }

    /// Spawn scrcpy -> FIFO -> GStreamer ingest producers and wait until real frames reach the bridge
    async fn spawn_external_producers(
        target_id: &str,
        source: CameraSource,
        orientation: Orientation,
        torch: bool,
        audio: bool,
    ) -> Result<(u32, Vec<u32>), String> {
        let fifo_path = stream_fifo_path();
        let _ = fs::remove_file(&fifo_path);
        nix::unistd::mkfifo(&fifo_path, nix::sys::stat::Mode::from_bits_truncate(0o600))
            .map_err(|e| format!("{}: {}", tr("Failed to create FIFO channel", "Не удалось создать FIFO канал"), e))?;
        let fifo_str = fifo_path.to_string_lossy().to_string();

        let mut gst_args = vec![
            "filesrc".to_string(),
            format!("location={}", fifo_str),
            "!".to_string(),
            "decodebin".to_string(),
        ];
        Self::append_ingest_producer_args(&mut gst_args);

        let gst_pid = match Self::spawn_gst_producer(gst_args).await {
            Ok(pid) => pid,
            Err(err) => return Err(err),
        };

        let mut scrcpy_cmd = tokio::process::Command::new("scrcpy");
        scrcpy_cmd
            .arg("-s")
            .arg(target_id)
            .arg("--video-source=camera")
            .arg(format!("--camera-size={}", orientation.resolution()))
            .arg("--camera-fps=30")
            .arg(format!(
                "--capture-orientation={}",
                orientation.scrcpy_angle(source)
            ))
            .arg("--video-codec-options=i-frame-interval=1")
            .arg("--no-window")
            .arg("--no-audio")
            .arg(format!("--record={}", fifo_str))
            .arg("--record-format=mkv");

        match source {
            CameraSource::ExternalBack => {
                scrcpy_cmd.arg("--camera-facing=back");
                if torch {
                    scrcpy_cmd.arg("--camera-torch");
                }
            }
            CameraSource::ExternalFront => {
                scrcpy_cmd.arg("--camera-facing=front");
            }
            CameraSource::Builtin => unreachable!(),
        }

        let (scrcpy_out, scrcpy_err) = Self::open_log_pair(&scrcpy_log_file());
        scrcpy_cmd
            .stdin(Stdio::null())
            .stdout(scrcpy_out)
            .stderr(scrcpy_err)
            .process_group(0);

        let mut scrcpy_child = match scrcpy_cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                Self::kill_pid_gracefully(gst_pid).await;
                return Err(format!("{}: {}", tr("Failed to start scrcpy", "Не удалось запустить scrcpy"), e));
            }
        };
        let scrcpy_pid = scrcpy_child.id().unwrap_or(0);
        tokio::spawn(async move {
            let _ = scrcpy_child.wait().await;
        });

        let mut aux_pids = vec![scrcpy_pid];
        if audio {
            let mut audio_cmd = tokio::process::Command::new("scrcpy");
            audio_cmd
                .arg("-s")
                .arg(target_id)
                .arg("--no-video")
                .arg("--no-window")
                .arg("--audio-source=mic")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0);
            if let Ok(mut audio_child) = audio_cmd.spawn() {
                if let Some(apid) = audio_child.id() {
                    aux_pids.push(apid);
                }
                tokio::spawn(async move {
                    let _ = audio_child.wait().await;
                });
            }
        }

        if let Err(err) = Self::wait_for_external_producer(gst_pid, scrcpy_pid).await {
            for &apid in &aux_pids {
                Self::kill_pid_gracefully(apid).await;
            }
            Self::kill_pid_gracefully(gst_pid).await;
            return Err(err);
        }

        Ok((gst_pid, aux_pids))

    }

    /// Stop active input producers while keeping the persistent _bridge alive on /dev/video0
    pub async fn stop_stream() -> Result<(), String> {
        let session = Self::get_current_session();
        Self::clear_session();
        Self::stop_producers_only(&session).await;
        Self::cleanup_producer_files();
        sleep(Duration::from_millis(80)).await;
        Ok(())
    }

    /// Complete emergency reset: kills all producers, clears ADB connections,
    /// removes stale FIFOs/state, and restarts the persistent bridge to clean studio-black.
    pub async fn force_reset() -> Result<(), String> {
        let session = Self::get_current_session();
        Self::clear_session();
        Self::stop_producers_only(&session).await;

        // Force terminate any lingering camera-specific scrcpy or gst-launch processes
        Self::kill_by_pattern("scrcpy.*--video-source=camera").await;
        Self::kill_by_pattern("scrcpy.*--no-video.*--audio-source=mic").await;
        Self::kill_by_pattern("gst-launch-1.0.*fdsink").await;

        // Disconnect all ADB devices to clear hung/stale TCP transports
        let _ = tokio::process::Command::new("adb")
            .arg("disconnect")
            .output()
            .await;

        // Clean up FIFO and logs
        Self::cleanup_producer_files();

        // Restart inerium-camera-bridge service to guarantee clean state and black frame on /dev/video0
        let restart_out = tokio::process::Command::new("systemctl")
            .args(["--user", "restart", "inerium-camera-bridge.service"])
            .output()
            .await;

        if let Ok(out) = restart_out {
            if !out.status.success() {
                let _ = Self::ensure_bridge_ready().await;
            }
        } else {
            let _ = Self::ensure_bridge_ready().await;
        }

        sleep(Duration::from_millis(150)).await;
        Ok(())
    }

    /// Start or hot-swap camera producer while keeping persistent /dev/video0 bridge alive
    pub async fn start_stream(
        source: CameraSource,
        orientation: Orientation,
        torch: bool,
        audio: bool,
        custom_ip: Option<&str>,
    ) -> Result<CameraSessionInfo, String> {
        let v4l2_dev = DeviceManager::find_v4l2_loopback()
            .ok_or_else(|| tr("Virtual device PhoneWebcam (/dev/video0) not found.", "Виртуальное устройство PhoneWebcam (/dev/video0) не найдено.").to_string())?;

        // Validate input availability BEFORE tearing down existing producer/bridge
        let builtin_dev_path = if source == CameraSource::Builtin {
            match DeviceManager::detect_builtin_webcam() {
                BuiltinStatus::Available { device_path } => Some(device_path),
                BuiltinStatus::HardwareDisabled => {
                    return Err(tr("Built-in camera disabled via hardware switch Fn + F6.", "Встроенная камера отключена аппаратно клавишей Fn + F6.").to_string());
                }
                BuiltinStatus::NotFound => {
                    return Err(tr("Built-in laptop camera not found in system.", "Встроенная камера ноутбука не найдена в системе.").to_string());
                }
            }
        } else {
            None
        };

        let external_target = if source != CameraSource::Builtin {
            Some(DeviceManager::discover_external_device(custom_ip).await?)
        } else {
            None
        };

        let current_session = Self::get_current_session();
        let bridge_pid = Self::ensure_bridge_running(&v4l2_dev).await?;
        Self::stop_producers_only(&current_session).await;

        let _ = fs::write(gst_log_file(), "");
        let _ = fs::write(scrcpy_log_file(), "");

        if let Some(dev_path) = builtin_dev_path {
            let mut args = vec!["v4l2src".to_string(), format!("device={}", dev_path)];
            if DeviceManager::supports_mjpg_720p(&dev_path) {
                args.push("!".to_string());
                args.push("image/jpeg,width=1280,height=720,framerate=30/1".to_string());
                args.push("!".to_string());
                args.push("jpegdec".to_string());
            }
            Self::append_ingest_producer_args(&mut args);

            let gst_pid = match Self::spawn_gst_producer(args).await {
                Ok(pid) => pid,
                Err(err) => {
                    let _ = Self::stop_stream().await;
                    return Err(err);
                }
            };

            if let Err(err) = Self::wait_for_builtin_producer(gst_pid).await {
                Self::kill_pid_gracefully(gst_pid).await;
                let _ = Self::stop_stream().await;
                return Err(err);
            }

            let info = CameraSessionInfo {
                is_streaming: true,
                source,
                orientation,
                torch: false,
                audio: false,
                conn_type: Some(format!("{} ({} -> {})", tr("Built-in camera", "Встроенная камера"), dev_path, v4l2_dev)),
                v4l2_device: Some(v4l2_dev),
                bridge_pid: Some(bridge_pid),
                pid: Some(gst_pid),
                aux_pids: Vec::new(),
            };
            Self::save_session(&info);
            return Ok(info);
        }

        let (target_id, conn_desc) = external_target.expect("external target checked above");

        // Phone camera may refuse the first session right after (re)connecting ADB ("Camera disconnected"), retry once
        let mut attempt = 0;
        let (gst_pid, aux_pids) = loop {
            match Self::spawn_external_producers(&target_id, source, orientation, torch, audio).await {
                Ok(pids) => break pids,
                Err(err) => {
                    attempt += 1;
                    Self::stop_producers_only(&CameraSessionInfo::default()).await;
                    if attempt >= 2 {
                        Self::clear_session();
                        return Err(err);
                    }
                    sleep(Duration::from_millis(800)).await;
                }
            }
        };

        let info = CameraSessionInfo {
            is_streaming: true,
            source,
            orientation,
            torch,
            audio,
            conn_type: Some(conn_desc),
            v4l2_device: Some(v4l2_dev),
            bridge_pid: Some(bridge_pid),
            pid: Some(gst_pid),
            aux_pids,
        };

        Self::save_session(&info);
        Ok(info)
    }
}
