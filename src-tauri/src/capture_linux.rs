//! Wayland uses the portal-granted PipeWire remote, independent of XWayland.
use crate::{
    capture::{crop_frame, encode_preview, SharedFrame, Source, SourcePreview},
    domain::{Crop, VideoKind, VideoSelection},
};
use anyhow::{Context, Result};
use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
use ashpd::desktop::Session;
use image::RgbaImage;
use pipewire::{
    context::ContextRc,
    main_loop::MainLoopRc,
    properties::properties,
    spa::{
        self,
        param::{
            format::{FormatProperties, MediaSubtype, MediaType},
            video::{VideoFormat, VideoInfoRaw},
            ParamType,
        },
        pod::{self, serialize::PodSerializer, Pod},
        utils::{Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{StreamFlags, StreamRc, StreamState},
};
use std::{
    collections::HashMap,
    io::Cursor,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

pub type PortalSources = Mutex<HashMap<String, Arc<PortalGrant>>>;

pub struct PortalGrant {
    portal: Screencast,
    session: Arc<Session<Screencast>>,
    node: u32,
    is_window: bool,
}

impl Drop for PortalGrant {
    fn drop(&mut self) {
        let session = self.session.clone();
        tauri::async_runtime::spawn(async move {
            let _ = session.close().await;
        });
    }
}

impl PortalGrant {
    async fn select() -> Result<Self> {
        let portal = Screencast::new().await?;
        let session = Arc::new(portal.create_session(Default::default()).await?);
        let mut grant = Self {
            portal,
            session,
            node: 0,
            is_window: false,
        };
        let portal = &grant.portal;
        portal
            .select_sources(
                &grant.session,
                SelectSourcesOptions::default()
                    .set_sources(SourceType::MONITOR | SourceType::WINDOW)
                    .set_multiple(false)
                    .set_cursor_mode(CursorMode::EMBEDDED),
            )
            .await?
            .response()?;
        let selected = portal
            .start(&grant.session, None, Default::default())
            .await?
            .response()?;
        let stream = selected
            .streams()
            .first()
            .context("No screen or window selected")?;
        grant.node = stream.pipe_wire_node_id();
        grant.is_window = stream.source_type() == Some(SourceType::WINDOW);
        Ok(grant)
    }

    async fn remote(&self) -> Result<OwnedFd> {
        Ok(self
            .portal
            .open_pipe_wire_remote(&self.session, Default::default())
            .await?)
    }

    pub async fn attach(&self, command: &mut tokio::process::Command) -> Result<()> {
        let remote = self.remote().await?;
        command.env("MEETINGPAL_PIPEWIRE_FD", remote.as_raw_fd().to_string());
        command.env("MEETINGPAL_PIPEWIRE_NODE", self.node.to_string());
        // Only this recorder inherits the already granted remote. The parent keeps CLOEXEC.
        unsafe {
            command.pre_exec(move || {
                if libc::fcntl(remote.as_raw_fd(), libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(())
    }
}

#[derive(serde::Serialize)]
pub struct PortalPreview {
    source: Source,
    preview: Option<SourcePreview>,
}

#[tauri::command]
pub async fn select_portal_source(
    grants: tauri::State<'_, PortalSources>,
) -> Result<PortalPreview, String> {
    async {
        let grant = PortalGrant::select().await?;
        let node = grant.node;
        // Permission to record the whole source remains useful if no still arrives.
        let preview = if let Ok(remote) = grant.remote().await {
            tokio::task::spawn_blocking(move || -> Result<SourcePreview> {
                let image = Arc::new(Mutex::new(None));
                let output = image.clone();
                let deadline = Instant::now() + Duration::from_secs(5);
                read_stream(
                    remote,
                    node,
                    move || Instant::now() >= deadline,
                    move |frame| {
                        *output.lock().unwrap() = Some(frame);
                        Ok(true)
                    },
                )?;
                let frame = image
                    .lock()
                    .unwrap()
                    .take()
                    .context("No preview frame from the system picker")?;
                encode_preview(frame, true)
            })
            .await?
            .ok()
        } else {
            None
        };
        let id = format!("portal:{}", uuid::Uuid::new_v4());
        let source = Source {
            id: id.clone(),
            name: String::new(),
            kind: if grant.is_window { "window" } else { "screen" }.into(),
            is_primary: false,
            width: preview.as_ref().map_or(0, |p| p.width),
            height: preview.as_ref().map_or(0, |p| p.height),
        };
        grants.lock().unwrap().insert(id, Arc::new(grant));
        Ok::<_, anyhow::Error>(PortalPreview { source, preview })
    }
    .await
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn release_portal_source(id: String, grants: tauri::State<'_, PortalSources>) {
    grants.lock().unwrap().remove(&id);
}

pub fn portal_capture(
    selection: VideoSelection,
    latest: SharedFrame,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let capture = |remote, node, crop: Option<Crop>| {
        read_stream(
            remote,
            node,
            move || !running.load(Ordering::Relaxed),
            move |frame| {
                *latest.lock().unwrap() = Some(crop_frame(frame, crop.as_ref())?);
                Ok(false)
            },
        )
    };
    if selection
        .id
        .as_deref()
        .is_some_and(|id| id.starts_with("portal:"))
    {
        let fd: i32 = std::env::var("MEETINGPAL_PIPEWIRE_FD")
            .context("Select the portal source again")?
            .parse()?;
        let node = std::env::var("MEETINGPAL_PIPEWIRE_NODE")?.parse()?;
        anyhow::ensure!(fd > 2, "Invalid PipeWire descriptor");
        // attach transfers ownership via exec. Do not pass the descriptor on to FFmpeg.
        let inherited = unsafe { OwnedFd::from_raw_fd(fd) };
        let remote = inherited.try_clone()?;
        drop(inherited);
        return capture(remote, node, selection.crop);
    }
    // Older configurations select their source at recording start.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let grant = runtime.block_on(PortalGrant::select())?;
    let remote = runtime.block_on(grant.remote())?;
    capture(remote, grant.node, selection.crop)
}

fn read_stream(
    remote: OwnedFd,
    node: u32,
    should_stop: impl Fn() -> bool + 'static,
    mut on_frame: impl FnMut(RgbaImage) -> Result<bool> + 'static,
) -> Result<()> {
    pipewire::init();
    let main_loop = MainLoopRc::new(None)?;
    let context = ContextRc::new(&main_loop, None)?;
    let core = context.connect_fd_rc(remote, None)?;
    let stream = StreamRc::new(
        core,
        "MeetingPal",
        properties! {*pipewire::keys::MEDIA_TYPE=>"Video",*pipewire::keys::MEDIA_CATEGORY=>"Capture",*pipewire::keys::MEDIA_ROLE=>"Screen"},
    )?;
    let error = Arc::new(Mutex::new(None::<String>));
    let error_state = error.clone();
    let error_frame = error.clone();
    let loop_state = main_loop.clone();
    let loop_frame = main_loop.clone();
    let _listener = stream
        .add_local_listener_with_user_data(VideoInfoRaw::default())
        .state_changed(move |_, _, _, state| {
            if let StreamState::Error(message) = state {
                *error_state.lock().unwrap() = Some(message.to_string());
                loop_state.quit();
            }
        })
        .param_changed(|_, format, id, param| {
            if id == ParamType::Format.as_raw() {
                if let Some(param) = param {
                    let _ = format.parse(param);
                }
            }
        })
        .process(move |stream, format| {
            let result = (|| -> Result<()> {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return Ok(());
                };
                let Some(data) = buffer.datas_mut().first_mut() else {
                    return Ok(());
                };
                let size = format.size();
                let width = size.width as usize;
                let height = size.height as usize;
                if width == 0 || height == 0 || data.chunk().size() == 0 {
                    return Ok(());
                }
                anyhow::ensure!(
                    width <= 16384 && height <= 16384,
                    "Capture dimensions exceed the supported limit"
                );
                let stride = data.chunk().stride();
                let offset = data.chunk().offset() as usize;
                anyhow::ensure!(
                    stride > 0 && stride as usize >= width * 4,
                    "Unsupported video row stride"
                );
                let Some(bytes) = data.data() else {
                    return Ok(());
                };
                let mut rgba = vec![0u8; width * height * 4];
                let bgr = matches!(format.format(), VideoFormat::BGRx | VideoFormat::BGRA);
                anyhow::ensure!(
                    bgr || matches!(format.format(), VideoFormat::RGBA | VideoFormat::RGBx),
                    "Unsupported portal pixel format"
                );
                for row in 0..height {
                    let start = offset + row * stride as usize;
                    let end = start + width * 4;
                    anyhow::ensure!(end <= bytes.len(), "Truncated portal video buffer");
                    let output = &mut rgba[row * width * 4..(row + 1) * width * 4];
                    output.copy_from_slice(&bytes[start..end]);
                    for pixel in output.chunks_exact_mut(4) {
                        if bgr {
                            pixel.swap(0, 2);
                        }
                        pixel[3] = 255;
                    }
                }
                let frame = RgbaImage::from_raw(size.width, size.height, rgba)
                    .context("Invalid portal image")?;
                if on_frame(frame)? {
                    loop_frame.quit();
                }
                Ok(())
            })();
            if let Err(e) = result {
                *error_frame.lock().unwrap() = Some(e.to_string());
                loop_frame.quit();
            }
        })
        .register()?;
    let object = pod::object!(
        SpaTypes::ObjectParamFormat,
        ParamType::EnumFormat,
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pod::property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::BGRx,
            VideoFormat::BGRA,
            VideoFormat::RGBA,
            VideoFormat::RGBx
        ),
        pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            Rectangle {
                width: 1280,
                height: 720
            },
            Rectangle {
                width: 1,
                height: 1
            },
            Rectangle {
                width: 16384,
                height: 16384
            }
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: 10, denom: 1 },
            Fraction { num: 0, denom: 1 },
            Fraction { num: 60, denom: 1 }
        )
    );
    let encoded = PodSerializer::serialize(Cursor::new(Vec::new()), &pod::Value::Object(object))
        .map_err(|e| anyhow::anyhow!("PipeWire format: {e}"))?
        .0
        .into_inner();
    let mut parameters = [Pod::from_bytes(&encoded).context("Invalid PipeWire format")?];
    stream.connect(
        Direction::Input,
        Some(node),
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut parameters,
    )?;
    let loop_stop = main_loop.clone();
    let timer = main_loop.loop_().add_timer(move |_| {
        if should_stop() {
            loop_stop.quit();
        }
    });
    timer
        .update_timer(
            Some(Duration::from_millis(100)),
            Some(Duration::from_millis(100)),
        )
        .into_result()?;
    main_loop.run();
    let failure = error.lock().unwrap().take();
    if let Some(error) = failure {
        anyhow::bail!(error)
    }
    Ok(())
}
