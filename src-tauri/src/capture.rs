use crate::{
    domain::*,
    storage::{atomic_json, read_json},
};
use anyhow::{Context, Result};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub is_default: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureManifest {
    pub segments: Vec<MediaSegment>,
    pub duration_ms: u64,
    pub video_files: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSpec {
    pub folder: PathBuf,
    pub config: MeetingConfig,
    pub ffmpeg: Option<PathBuf>,
    pub test_seconds: Option<u64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CaptureEvent {
    Ready,
    Level {
        microphone: f32,
        desktop: f32,
        duration_ms: u64,
    },
    Segment {
        segment: MediaSegment,
    },
    Warning {
        message: String,
    },
    State {
        paused: bool,
    },
    Stopped {
        manifest: CaptureManifest,
    },
}
pub trait CaptureBackend {
    fn devices(&self) -> Result<Vec<Device>>;
    fn sources(&self) -> Result<Vec<Source>>;
}
pub struct NativeCapture;
impl CaptureBackend for NativeCapture {
    fn devices(&self) -> Result<Vec<Device>> {
        let devices: Vec<Device> = flexaudio::devices()?
            .into_iter()
            .map(|d| Device {
                id: d.id,
                name: d.name,
                kind: if d.is_loopback {
                    "desktop"
                } else {
                    "microphone"
                }
                .into(),
                is_default: d.is_default,
            })
            .collect();
        #[cfg(target_os = "macos")]
        {
            let mut devices = devices
                .into_iter()
                .filter(|d| d.kind == "microphone")
                .collect::<Vec<_>>();
            devices.push(Device {
                id: "sck-desktop".into(),
                name: "System audio (ScreenCaptureKit)".into(),
                kind: "desktop".into(),
                is_default: true,
            });
            Ok(devices)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(devices)
        }
    }
    fn sources(&self) -> Result<Vec<Source>> {
        sources()
    }
}
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn sources() -> Result<Vec<Source>> {
    Ok(scap::get_all_targets()
        .into_iter()
        .map(|target| {
            let (width, height) = scap::get_target_dimensions(&target);
            match target {
                scap::Target::Window(w) => Source {
                    id: w.id.to_string(),
                    name: w.title,
                    kind: "window".into(),
                    width: width as u32,
                    height: height as u32,
                },
                scap::Target::Display(d) => Source {
                    id: d.id.to_string(),
                    name: d.title,
                    kind: "screen".into(),
                    width: width as u32,
                    height: height as u32,
                },
            }
        })
        .collect())
}
#[cfg(target_os = "linux")]
fn sources() -> Result<Vec<Source>> {
    if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") {
        return Ok(vec![Source {
            id: "portal".into(),
            name: "System screen / window picker".into(),
            kind: "portal".into(),
            width: 0,
            height: 0,
        }]);
    }
    let mut out = vec![];
    for m in xcap::Monitor::all()? {
        out.push(Source {
            id: m.id()?.to_string(),
            name: m.name()?,
            kind: "screen".into(),
            width: m.width()?,
            height: m.height()?,
        });
    }
    for w in xcap::Window::all()? {
        if !w.is_minimized()? {
            out.push(Source {
                id: w.id()?.to_string(),
                name: w.title()?,
                kind: "window".into(),
                width: w.width()?,
                height: w.height()?,
            });
        }
    }
    Ok(out)
}
pub fn hidden_command(path: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}
pub(crate) fn emit(event: CaptureEvent) {
    if let Ok(json) = serde_json::to_string(&event) {
        println!("{json}");
        let _ = std::io::stdout().flush();
    }
}
fn warning(message: impl Into<String>) {
    emit(CaptureEvent::Warning {
        message: message.into(),
    });
}

pub(crate) struct LaneChunk {
    pub data: Vec<f32>,
    pub frames: usize,
    pub frame_index: u64,
    pub peak: f32,
    pub dropped_before: u32,
    pub discontinuity: bool,
}
enum AudioInput {
    Native(flexaudio::Stream),
    #[cfg(target_os = "macos")]
    Screen(crate::capture_mac_audio::MacAudio),
}
impl AudioInput {
    fn start(&mut self) -> Result<()> {
        match self {
            Self::Native(s) => Ok(s.start()?),
            #[cfg(target_os = "macos")]
            Self::Screen(s) => s.start(),
        }
    }
    fn stop(&mut self) {
        match self {
            Self::Native(s) => {
                s.stop();
            }
            #[cfg(target_os = "macos")]
            Self::Screen(s) => s.stop(),
        }
    }
    fn poll_chunk(&mut self) -> Option<LaneChunk> {
        match self {
            Self::Native(s) => s.poll_chunk().map(|c| LaneChunk {
                data: c.data,
                frames: c.frames,
                frame_index: c.frame_index,
                peak: c.peak,
                dropped_before: c.dropped_before,
                discontinuity: c.flags.intersects(
                    flexaudio::ChunkFlags::DISCONTINUITY | flexaudio::ChunkFlags::RECOVERED,
                ),
            }),
            #[cfg(target_os = "macos")]
            Self::Screen(s) => s.poll_chunk(),
        }
    }
    fn poll_event(&mut self) -> Option<String> {
        match self {
            Self::Native(s) => s.poll_event().map(|e| format!("{e:?}")),
            #[cfg(target_os = "macos")]
            Self::Screen(s) => s.poll_event(),
        }
    }
}
struct AudioLane {
    stream: AudioInput,
    samples: Vec<f32>,
    anchor: Option<(u64, f64)>,
    peak: f32,
    last_data: Instant,
    received: bool,
    warned: bool,
}
impl AudioLane {
    fn open(kind: flexaudio::SourceKind, device_id: Option<String>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        if matches!(kind, flexaudio::SourceKind::SystemLoopback) {
            anyhow::ensure!(
                device_id.as_deref().is_none_or(|id| id == "sck-desktop"),
                "ScreenCaptureKit captures system audio. Select the system audio source."
            );
            return Ok(Self::from_input(AudioInput::Screen(
                crate::capture_mac_audio::MacAudio::new(),
            )));
        }
        let stream = flexaudio::open(flexaudio::StreamConfig {
            kind,
            device_id,
            silence_probe: false,
            output: flexaudio::OutputFormat {
                sample_rate: 48_000,
                channels: 1,
            },
            ring_capacity_chunks: 500,
            ..Default::default()
        })?;
        Ok(Self::from_input(AudioInput::Native(stream)))
    }
    fn from_input(stream: AudioInput) -> Self {
        Self {
            stream,
            samples: Vec::new(),
            anchor: None,
            peak: 0.0,
            last_data: Instant::now(),
            received: false,
            warned: false,
        }
    }
    fn drain(&mut self, active_ms: u64, committed: u64, paused: bool, name: &str) {
        while let Some(chunk) = self.stream.poll_chunk() {
            if chunk.discontinuity || self.last_data.elapsed() > Duration::from_millis(500) {
                self.anchor = None;
            }
            self.last_data = Instant::now();
            self.received = true;
            self.warned = false;
            if paused {
                self.anchor = None;
                continue;
            }
            self.peak = self.peak.max(chunk.peak);
            let observed = active_ms as f64 * 48.0 - chunk.frames as f64;
            let anchor = self
                .anchor
                .get_or_insert((chunk.frame_index, observed.max(0.0)));
            let delta = chunk.frame_index.saturating_sub(anchor.0) as f64;
            let prediction = anchor.1 + delta;
            // Slowly correct independent hardware clocks; do not turn scheduling jitter into gaps.
            let drift = observed - prediction;
            if drift.abs() > 960.0 {
                anchor.1 += drift.clamp(-4800.0, 4800.0) * 0.002;
            }
            let start = (anchor.1 + delta).max(0.0) as u64;
            for (offset, value) in chunk.data.iter().enumerate() {
                let at = start + offset as u64;
                if at < committed {
                    continue;
                }
                let index = (at - committed) as usize;
                if index > 48_000 * 15 {
                    continue;
                }
                if index >= self.samples.len() {
                    self.samples.resize(index + 1, 0.0);
                }
                self.samples[index] = if value.is_finite() { *value } else { 0.0 };
            }
            if chunk.dropped_before > 0 {
                warning(format!(
                    "{name}: {} audio chunks were lost; a gap has been preserved.",
                    chunk.dropped_before
                ));
            }
        }
        while let Some(event) = self.stream.poll_event() {
            let description = event;
            if description.contains("Error")
                || description.contains("Loss")
                || description.contains("Disconnected")
                || description.contains("Recover")
            {
                warning(format!("{name}: {description}"));
            }
        }
        if !paused && self.last_data.elapsed() > Duration::from_secs(3) && !self.warned {
            self.warned = true;
            warning(format!("{name}: no audio samples received for three seconds. Check the device and recording permission."));
        }
    }
    fn take(&mut self, count: usize) -> Vec<f32> {
        if self.samples.len() < count {
            self.samples.resize(count, 0.0);
        }
        self.samples.drain(..count).collect()
    }
}
pub fn write_wav(path: &Path, samples: &[f32], rate: u32) -> Result<()> {
    let mut wav = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for sample in samples {
        wav.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    wav.finalize()?;
    fs::OpenOptions::new().write(true).open(path)?.sync_all()?;
    Ok(())
}
pub fn mix_audio(mic: &[f32], desktop: &[f32], echo_control: bool) -> Vec<f32> {
    // Conservative echo attenuation only on the derived mix. Original tracks are untouched.
    let mut gain = 1.0f32;
    if echo_control && mic.len() == desktop.len() && !mic.is_empty() {
        let energy: f32 = desktop.iter().map(|v| v * v).sum();
        let mic_energy: f32 = mic.iter().map(|v| v * v).sum();
        let dot: f32 = mic.iter().zip(desktop).map(|(a, b)| a * b).sum();
        if energy > 0.01 && dot / (energy * mic_energy).sqrt().max(0.001) > 0.85 {
            gain = 0.15;
        }
    }
    mic.iter()
        .zip(desktop)
        .map(|(a, b)| (a * gain + b).clamp(-1.0, 1.0))
        .collect()
}
fn commit_audio(
    spec: &WorkerSpec,
    manifest: &mut CaptureManifest,
    mic: &mut AudioLane,
    sys: &mut AudioLane,
    count: usize,
) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let index = manifest.segments.len();
    let microphone = format!("microphone-{index:06}.wav");
    let desktop = format!("desktop-{index:06}.wav");
    let mixed = format!("mixed-{index:06}.wav");
    let mic_data = mic.take(count);
    let sys_data = sys.take(count);
    write_wav(&spec.folder.join(&microphone), &mic_data, 48_000)?;
    write_wav(&spec.folder.join(&desktop), &sys_data, 48_000)?;
    let mixed_data = mix_audio(&mic_data, &sys_data, spec.config.denoise);
    // Three-sample box filter before 48→16k decimation; ASR receives normalized mono PCM.
    let downsampled: Vec<f32> = mixed_data
        .chunks(3)
        .map(|c| c.iter().sum::<f32>() / c.len() as f32)
        .collect();
    write_wav(&spec.folder.join(&mixed), &downsampled, 16_000)?;
    let start_ms = manifest.duration_ms;
    let end_ms = start_ms + count as u64 / 48;
    let segment = MediaSegment {
        id: format!("audio-{index:06}"),
        start_ms,
        end_ms,
        microphone: Some(microphone),
        desktop: Some(desktop),
        mixed,
    };
    manifest.segments.push(segment.clone());
    manifest.duration_ms = end_ms;
    atomic_json(&spec.folder.join("capture.json"), manifest)?;
    emit(CaptureEvent::Segment { segment });
    Ok(())
}
pub fn crop_frame(frame: RgbaImage, crop: Option<&Crop>) -> Result<RgbaImage> {
    if let Some(c) = crop {
        anyhow::ensure!(c.width > 0 && c.height > 0 && c.x.checked_add(c.width).is_some_and(|x| x <= frame.width()) && c.y.checked_add(c.height).is_some_and(|y| y <= frame.height()), "The capture region is outside the selected source. Video capture stopped to avoid recording unintended content.");
        Ok(image::imageops::crop_imm(&frame, c.x, c.y, c.width, c.height).to_image())
    } else {
        Ok(frame)
    }
}
#[doc(hidden)]
pub type SharedFrame = Arc<Mutex<Option<RgbaImage>>>;
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn capture_frames(
    selection: VideoSelection,
    latest: SharedFrame,
    running: Arc<AtomicBool>,
) -> Result<()> {
    if !scap::has_permission() {
        anyhow::ensure!(
            scap::request_permission(),
            "Screen recording permission was not granted."
        );
    }
    let target = scap::get_all_targets()
        .into_iter()
        .find(|t| match t {
            scap::Target::Window(w) => {
                selection.kind == VideoKind::Window
                    && selection.id.as_deref() == Some(&w.id.to_string())
            }
            scap::Target::Display(d) => {
                selection.kind != VideoKind::Window
                    && selection.id.as_deref() == Some(&d.id.to_string())
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Selected screen or window is no longer available."))?;
    let mut capturer = scap::capturer::Capturer::build(scap::capturer::Options {
        fps: 10,
        show_cursor: true,
        show_highlight: true,
        target: Some(target),
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::Captured,
        captures_audio: false,
        ..Default::default()
    })?;
    capturer.start_capture();
    while running.load(Ordering::Relaxed) {
        let frame = capturer.get_next_frame()?;
        let converted = match frame {
            scap::frame::Frame::Video(scap::frame::VideoFrame::BGRA(f)) => {
                let mut bytes = f.data;
                for p in bytes.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
                RgbaImage::from_raw(f.width as u32, f.height as u32, bytes)
            }
            scap::frame::Frame::Video(scap::frame::VideoFrame::BGR0(f)) => {
                let mut bytes = f.data;
                for p in bytes.chunks_exact_mut(4) {
                    p.swap(0, 2);
                    p[3] = 255;
                }
                RgbaImage::from_raw(f.width as u32, f.height as u32, bytes)
            }
            _ => None,
        };
        if let Some(frame) = converted {
            *latest.lock().unwrap() = Some(crop_frame(frame, selection.crop.as_ref())?);
        }
    }
    capturer.stop_capture();
    Ok(())
}
#[cfg(target_os = "linux")]
fn capture_frames(
    selection: VideoSelection,
    latest: SharedFrame,
    running: Arc<AtomicBool>,
) -> Result<()> {
    if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") {
        return crate::capture_linux::portal_capture(selection, latest, running);
    }
    if selection.kind == VideoKind::Window
        && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
    {
        let window = xcap::Window::all()?
            .into_iter()
            .find(|w| w.id().ok().map(|id| id.to_string()) == selection.id)
            .context("Selected window is unavailable")?;
        while running.load(Ordering::Relaxed) {
            *latest.lock().unwrap() = Some(crop_frame(
                window.capture_image()?,
                selection.crop.as_ref(),
            )?);
            thread::sleep(Duration::from_millis(100));
        }
    } else {
        let monitors = xcap::Monitor::all()?;
        let monitor = monitors
            .iter()
            .find(|m| m.id().ok().map(|id| id.to_string()) == selection.id)
            .context("The selected display is no longer available")?;
        let (recorder, receiver) = monitor.video_recorder()?;
        recorder.start()?;
        while running.load(Ordering::Relaxed) {
            if let Ok(frame) = receiver.recv_timeout(Duration::from_millis(200)) {
                if let Some(frame) = RgbaImage::from_raw(frame.width, frame.height, frame.raw) {
                    *latest.lock().unwrap() = Some(crop_frame(frame, selection.crop.as_ref())?);
                }
            }
        }
        recorder.stop()?;
    }
    Ok(())
}
#[doc(hidden)]
pub fn encode_video(
    ffmpeg: PathBuf,
    folder: PathBuf,
    latest: SharedFrame,
    active: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let first = latest
        .lock()
        .unwrap()
        .clone()
        .context("No video frame received")?;
    let width = (first.width().min(1920) / 2 * 2).max(2);
    let height =
        ((first.height() as f64 * width as f64 / first.width() as f64) as u32 / 2 * 2).max(2);
    let size = format!("{width}x{height}");
    let log = fs::File::create(folder.join("encoder.log"))?;
    let mut encoder = hidden_command(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-video_size",
            &size,
            "-framerate",
            "10",
            "-i",
            "pipe:0",
            "-an",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libvpx",
            "-deadline",
            "realtime",
            "-cpu-used",
            "8",
            "-b:v",
            "1500k",
            "-g",
            "50",
            "-f",
            "segment",
            "-segment_time",
            "5",
            "-reset_timestamps",
            "1",
        ])
        .arg(folder.join("video-%06d.webm"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .context("Could not start the bundled video encoder")?;
    let mut input = encoder.stdin.take().unwrap();
    let mut frames = 0u64;
    loop {
        let required = active.load(Ordering::Relaxed) / 100;
        if frames < required {
            let frame = latest
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| first.clone());
            let resized = image::imageops::resize(
                &frame,
                width,
                height,
                image::imageops::FilterType::Triangle,
            );
            input
                .write_all(resized.as_raw())
                .context("Video encoder stopped; audio recording continues")?;
            frames += 1;
        } else if !running.load(Ordering::Relaxed) {
            break;
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
    drop(input);
    anyhow::ensure!(
        encoder.wait()?.success(),
        "Video encoder failed. See local encoder.log."
    );
    Ok(())
}
fn completed_video(folder: &Path, finalizing: bool) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|s| s.starts_with("video-") && s.ends_with(".webm"))
        .collect();
    files.sort();
    if !finalizing {
        files.pop();
    }
    files
}
pub fn run_worker(spec_path: &Path) -> Result<()> {
    let spec: WorkerSpec = read_json(spec_path)?;
    let running = Arc::new(AtomicBool::new(true));
    let paused = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicU64::new(0));
    let input_running = running.clone();
    let input_paused = paused.clone();
    thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line.unwrap_or_default().trim() {
                "stop" => break,
                "pause" => {
                    input_paused.store(true, Ordering::SeqCst);
                    emit(CaptureEvent::State { paused: true });
                }
                "resume" => {
                    input_paused.store(false, Ordering::SeqCst);
                    emit(CaptureEvent::State { paused: false });
                }
                _ => {}
            }
        }
        input_running.store(false, Ordering::SeqCst);
    });
    let latest: SharedFrame = Arc::new(Mutex::new(None));
    let video_failed = Arc::new(AtomicBool::new(false));
    if spec.config.video.kind != VideoKind::None {
        anyhow::ensure!(spec.ffmpeg.is_some(), "Bundled FFmpeg runtime is missing.");
        let latest_copy = latest.clone();
        let live = running.clone();
        let failed = video_failed.clone();
        let selection = spec.config.video.clone();
        thread::spawn(move || {
            if let Err(error) = capture_frames(selection, latest_copy.clone(), live) {
                warning(format!(
                    "Video source lost: {error}. Audio continues; missing video is black."
                ));
                let mut frame = latest_copy.lock().unwrap();
                if let Some(previous) = frame.as_ref() {
                    *frame = Some(RgbaImage::from_pixel(
                        previous.width(),
                        previous.height(),
                        image::Rgba([0, 0, 0, 255]),
                    ));
                }
                failed.store(true, Ordering::SeqCst);
            }
        });
        let started = Instant::now();
        while latest.lock().unwrap().is_none()
            && running.load(Ordering::Relaxed)
            && !video_failed.load(Ordering::Relaxed)
            && started.elapsed() < Duration::from_secs(45)
        {
            thread::sleep(Duration::from_millis(50));
        }
        anyhow::ensure!(
            latest.lock().unwrap().is_some(),
            "No screen frames received. Check screen recording permission."
        );
    }
    let mut microphone =
        AudioLane::open(flexaudio::SourceKind::Mic, spec.config.microphone.clone())
            .context("Microphone could not be opened")?;
    let mut desktop = AudioLane::open(
        flexaudio::SourceKind::SystemLoopback,
        spec.config.system_device.clone(),
    )
    .context("Desktop audio could not be opened")?;
    microphone.stream.start()?;
    desktop.stream.start()?;
    // Starting one native device can take longer than the other. Discard setup-time
    // audio before establishing the common origin so it cannot offset the tracks.
    microphone.drain(0, 0, true, "Microphone");
    desktop.drain(0, 0, true, "Desktop audio");
    let video_worker = if let Some(ffmpeg) = spec
        .ffmpeg
        .clone()
        .filter(|_| spec.config.video.kind != VideoKind::None)
    {
        let folder = spec.folder.clone();
        let live = running.clone();
        let timeline = active.clone();
        Some(thread::spawn(move || {
            let result = encode_video(ffmpeg, folder, latest, timeline, live);
            if let Err(error) = &result {
                warning(format!("Video encoding stopped: {error}. Audio continues."));
            }
            result
        }))
    } else {
        None
    };
    let mut manifest = CaptureManifest::default();
    atomic_json(&spec.folder.join("capture.json"), &manifest)?;
    emit(CaptureEvent::Ready);
    let mut last = Instant::now();
    let mut active_duration = Duration::ZERO;
    let mut meter_at = Instant::now();
    let mut space_at = Instant::now();
    while running.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now.duration_since(last) > Duration::from_secs(2) && !paused.load(Ordering::SeqCst) {
            paused.store(true, Ordering::SeqCst);
            warning("Recording paused after a system suspension or long scheduling gap. Check your devices, then resume.");
            emit(CaptureEvent::State { paused: true });
        }
        let is_paused = paused.load(Ordering::SeqCst);
        if !is_paused {
            active_duration += now.duration_since(last);
        }
        last = now;
        let active_ms = active_duration.as_millis() as u64;
        active.store(active_ms, Ordering::Relaxed);
        let committed = manifest.duration_ms * 48;
        microphone.drain(active_ms, committed, is_paused, "Microphone");
        desktop.drain(active_ms, committed, is_paused, "Desktop audio");
        if active_ms >= manifest.duration_ms + SEGMENT_MS + 250 {
            commit_audio(
                &spec,
                &mut manifest,
                &mut microphone,
                &mut desktop,
                SEGMENT_MS as usize * 48,
            )?;
            manifest.video_files = completed_video(&spec.folder, false);
            atomic_json(&spec.folder.join("capture.json"), &manifest)?;
        }
        if meter_at.elapsed() > Duration::from_millis(150) {
            emit(CaptureEvent::Level {
                microphone: microphone.peak,
                desktop: desktop.peak,
                duration_ms: active_ms,
            });
            microphone.peak = 0.0;
            desktop.peak = 0.0;
            meter_at = Instant::now();
        }
        if space_at.elapsed() > Duration::from_secs(5) {
            if fs2::available_space(&spec.folder)? < 100 * 1024 * 1024 {
                warning("Less than 100 MB free space. Stopping safely.");
                break;
            }
            space_at = Instant::now();
        }
        if spec.test_seconds.is_some_and(|s| active_ms >= s * 1000) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    running.store(false, Ordering::SeqCst);
    microphone.stream.stop();
    desktop.stream.stop();
    let remaining_ms = (active_duration.as_millis() as u64).saturating_sub(manifest.duration_ms);
    commit_audio(
        &spec,
        &mut manifest,
        &mut microphone,
        &mut desktop,
        remaining_ms as usize * 48,
    )?;
    if let Some(video) = video_worker {
        if let Ok(Err(error)) = video.join() {
            warning(error.to_string());
        }
    }
    manifest.video_files = completed_video(&spec.folder, true);
    atomic_json(&spec.folder.join("capture.json"), &manifest)?;
    emit(CaptureEvent::Stopped { manifest });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn committed_wav_can_be_flushed_and_reopened() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("segment.wav");
        write_wav(&file, &vec![0.2; 48000], 48000).unwrap();
        let reader = hound::WavReader::open(file).unwrap();
        assert_eq!(reader.duration(), 48000);
        assert_eq!(reader.spec().sample_rate, 48000);
    }
    #[test]
    fn crop_never_leaks_pixels_outside_selection() {
        let mut image = RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255]));
        image.put_pixel(3, 4, image::Rgba([0, 255, 0, 255]));
        let crop = Crop {
            x: 3,
            y: 4,
            width: 1,
            height: 1,
        };
        let result = crop_frame(image.clone(), Some(&crop)).unwrap();
        assert_eq!(result.as_raw(), &[0, 255, 0, 255]);
        assert!(crop_frame(
            image,
            Some(&Crop {
                x: u32::MAX,
                ..crop
            })
        )
        .is_err());
    }
    #[test]
    fn originals_are_not_changed_by_mix() {
        let a = vec![0.3; 100];
        let b = vec![0.4; 100];
        let mixed = mix_audio(&a, &b, true);
        assert_eq!(a[0], 0.3);
        assert_eq!(b[0], 0.4);
        assert!(mixed[0] < 0.7);
    }
}
