//! Wayland uses the portal-granted PipeWire remote, independent of XWayland.
use crate::{
    capture::{crop_frame, SharedFrame},
    domain::VideoSelection,
};
use anyhow::{Context, Result};
use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
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
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub fn portal_capture(
    selection: VideoSelection,
    latest: SharedFrame,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let (_portal, _session, remote, node) = runtime.block_on(async {
        let portal = Screencast::new().await?;
        let session = portal.create_session(Default::default()).await?;
        portal
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_sources(SourceType::MONITOR | SourceType::WINDOW)
                    .set_multiple(false)
                    .set_cursor_mode(CursorMode::EMBEDDED),
            )
            .await?
            .response()?;
        let selected = portal
            .start(&session, None, Default::default())
            .await?
            .response()?;
        let node = selected
            .streams()
            .first()
            .context("No screen or window selected")?
            .pipe_wire_node_id();
        let fd = portal
            .open_pipe_wire_remote(&session, Default::default())
            .await?;
        Ok::<_, anyhow::Error>((portal, session, fd, node))
    })?;
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
                *latest.lock().unwrap() = Some(crop_frame(frame, selection.crop.as_ref())?);
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
        if !running.load(Ordering::Relaxed) {
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
