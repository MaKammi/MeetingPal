use meetingpal_lib::{
    domain::*,
    media::Runtimes,
    models::Models,
    providers::{Ai, AnalysisProvider, TranscriptionProvider},
    secrets::Secrets,
    storage::Store,
    AppState,
};
use std::{path::PathBuf, sync::Arc};

/// Opt-in: downloads ~3 GB of public models; never sends meeting data to a provider.
#[tokio::test]
#[ignore = "downloads real models and requires bundled local AI runtimes"]
async fn downloaded_models_transcribe_and_analyze_real_fixture() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let library = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        store: Store::open(library.path().to_path_buf()).unwrap(),
        models: Models::new(root.join("runtime-data/test-models")),
        runtimes: Runtimes {
            directory: root.join("src-tauri/binaries"),
        },
        secrets: Secrets::default(),
        ai: Ai::new().unwrap(),
        recording: tokio::sync::Mutex::new(None),
    });
    for id in ["whisper-small", "qwen-text"] {
        state
            .models
            .download_with_progress(id.into(), |_| {})
            .await
            .unwrap();
        assert!(state.models.spec(id).unwrap().installed);
    }
    let mut meeting = Meeting::new(MeetingConfig {
        title: "JFK test fixture".into(),
        language: "en".into(),
        consent: true,
        ..Default::default()
    });
    state.store.save(&meeting).unwrap();
    let folder = state.store.folder(&meeting.id).unwrap();
    std::fs::copy(
        root.join("resources/benchmark.wav"),
        folder.join("input.wav"),
    )
    .unwrap();
    let reader = hound::WavReader::open(folder.join("input.wav")).unwrap();
    let segment = MediaSegment {
        id: "fixture".into(),
        start_ms: 0,
        end_ms: reader.duration() as u64 * 1000 / reader.spec().sample_rate as u64,
        microphone: None,
        desktop: None,
        mixed: "input.wav".into(),
    };
    meeting.transcript = state
        .ai
        .transcribe(&state, &meeting, &segment)
        .await
        .unwrap();
    let words = meeting
        .transcript
        .iter()
        .map(|s| s.text.clone())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        words.contains("country"),
        "Expected recognized speech from the public fixture"
    );
    state.store.save(&meeting).unwrap();
    meeting.media.push(segment);
    state.store.save(&meeting).unwrap();
    meetingpal_lib::jobs::refine_transcript(&state, &meeting, || {})
        .await
        .unwrap();
    meeting = state.store.load(&meeting.id).unwrap();
    assert!(!meeting.transcript.is_empty());
    assert!(meeting
        .transcript
        .iter()
        .all(|s| s.id.starts_with("final-") && !s.provisional));
    let refined = serde_json::to_value(&meeting.transcript).unwrap();
    meetingpal_lib::jobs::refine_transcript(&state, &meeting, || {})
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(state.store.load(&meeting.id).unwrap().transcript).unwrap(),
        refined
    );
    let analysis = state.ai.analyze(&state, &meeting, false).await.unwrap();
    assert!(!analysis.summary.is_empty());
    analysis.validate_sources(&meeting.source_ids()).unwrap();
    let repeated = state.ai.analyze(&state, &meeting, false).await.unwrap();
    assert_eq!(
        serde_json::to_value(&analysis.summary).unwrap(),
        serde_json::to_value(&repeated.summary).unwrap()
    );
    let answer = state
        .ai
        .answer(&state, &meeting, "What does the speaker ask people to do?")
        .await
        .unwrap();
    assert!(!answer.text.is_empty());
    assert!(!answer.sources.is_empty());
    meeting.transcript[0].text = "This is a manual correction.".into();
    meeting.transcript[0].edited = true;
    state.store.save(&meeting).unwrap();
    meetingpal_lib::jobs::refine_transcript(&state, &meeting, || {})
        .await
        .unwrap();
    assert_eq!(
        state.store.load(&meeting.id).unwrap().transcript[0].text,
        meeting.transcript[0].text
    );
    assert_eq!(state.store.load(&meeting.id).unwrap().cloud_requests, 0);
    state.ai.shutdown().await;
}
