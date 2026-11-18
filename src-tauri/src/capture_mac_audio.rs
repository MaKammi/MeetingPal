//! System audio on macOS 14.2+, including audio-only recordings.
//! ScreenCaptureKit uses a 1x1 screen source; video buffers are discarded in memory.
use crate::capture::LaneChunk;
use anyhow::{Context, Result};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

pub struct MacAudio {
    running: Arc<AtomicBool>,
    chunks: Option<mpsc::Receiver<LaneChunk>>,
    events: Option<mpsc::Receiver<String>>,
}
impl MacAudio {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            chunks: None,
            events: None,
        }
    }
    pub fn start(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::sync_channel(500);
        let (events_tx, events_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);
        self.chunks = Some(rx);
        self.events = Some(events_rx);
        thread::spawn(move || {
            let result = (|| -> Result<()> {
                anyhow::ensure!(
                    scap::has_permission() || scap::request_permission(),
                    "ScreenCaptureKit system audio requires screen-recording permission."
                );
                let mut capture = scap::capturer::Capturer::build(scap::capturer::Options {
                    fps: 1,
                    captures_audio: true,
                    crop_area: Some(scap::capturer::Area {
                        origin: scap::capturer::Point { x: 0., y: 0. },
                        size: scap::capturer::Size {
                            width: 1.,
                            height: 1.,
                        },
                    }),
                    output_type: scap::frame::FrameType::BGRAFrame,
                    ..Default::default()
                })?;
                capture.start_capture();
                let _ = ready_tx.send(Ok::<(), String>(()));
                let mut pending = Vec::new();
                let mut frame_index = 0;
                let mut dropped = 0;
                while running.load(Ordering::Relaxed) {
                    if let scap::frame::Frame::Audio(frame) = capture.get_next_frame()? {
                        anyhow::ensure!(
                            matches!(frame.format(), scap::frame::AudioFormat::F32)
                                && frame.rate() == 48_000
                                && frame.channels() > 0,
                            "Unsupported ScreenCaptureKit audio format"
                        );
                        let channels = frame.channels() as usize;
                        let samples: Vec<f32> = frame
                            .raw_data()
                            .chunks_exact(4)
                            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                            .collect();
                        for i in 0..frame.sample_count() {
                            let value = (0..channels)
                                .map(|ch| {
                                    samples[if frame.is_planar() {
                                        ch * frame.sample_count() + i
                                    } else {
                                        i * channels + ch
                                    }]
                                })
                                .sum::<f32>()
                                / channels as f32;
                            pending.push(value);
                        }
                        while pending.len() >= 960 {
                            let data = pending.drain(..960).collect::<Vec<_>>();
                            let peak = data.iter().fold(0f32, |a, v| a.max(v.abs()));
                            let chunk = LaneChunk {
                                data,
                                frames: 960,
                                frame_index,
                                peak,
                                dropped_before: dropped,
                            };
                            frame_index += 960;
                            match tx.try_send(chunk) {
                                Ok(()) => dropped = 0,
                                Err(mpsc::TrySendError::Full(_)) => dropped += 1,
                                Err(mpsc::TrySendError::Disconnected(_)) => break,
                            }
                        }
                    }
                }
                capture.stop_capture();
                Ok(())
            })();
            if let Err(error) = result {
                let error = format!("Error: {error}");
                let _ = ready_tx.send(Err(error.clone()));
                let _ = events_tx.send(error);
            }
        });
        ready_rx
            .recv_timeout(Duration::from_secs(60))
            .context("ScreenCaptureKit audio permission timed out")?
            .map_err(anyhow::Error::msg)
    }
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
    pub fn poll_chunk(&mut self) -> Option<LaneChunk> {
        self.chunks.as_ref()?.try_recv().ok()
    }
    pub fn poll_event(&mut self) -> Option<String> {
        self.events.as_ref()?.try_recv().ok()
    }
}
impl Drop for MacAudio {
    fn drop(&mut self) {
        self.stop();
    }
}
