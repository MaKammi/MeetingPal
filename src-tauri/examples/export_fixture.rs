//! Synthetic export fixture for visual inspection; no capture, models or network.
use meetingpal_lib::{domain::*, exports};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test-results/exports");
    std::fs::create_dir_all(&folder)?;
    let mut meeting = Meeting::new(MeetingConfig {
        title: "MeetingPal – Qualität und nächste Schritte".into(),
        language: "de".into(),
        ..Default::default()
    });
    meeting.duration_ms = 300_000;
    meeting.notes = "Synthetische Daten für die Exportprüfung. Umlaute: ä ö ü Ä Ö Ü ß.\nKeine Aufnahme echter Teilnehmer.".into();
    meeting.transcript = vec![
        TranscriptSegment { id: "audio-0".into(), start_ms: 0, end_ms: 15000, text: "Wir prüfen zuerst die Audioqualität. Kim übernimmt den Test auf Windows. Einen Termin haben wir noch nicht vereinbart.".into(), speaker: Some("Alex".into()), provisional: false, edited: false },
        TranscriptSegment { id: "audio-1".into(), start_ms: 15000, end_ms: 30000, text: "Die Folie zeigt unsere nächsten Schritte: Linux-Aufnahme testen, Quellen prüfen und das Testpaket vorbereiten.".into(), speaker: Some("Kim".into()), provisional: false, edited: false },
    ];
    meeting.visuals.push(VisualEvidence { id: "visual-0".into(), at_ms: 20000, text: "Die Präsentation nennt die drei Arbeitspakete Linux-Aufnahme, Quellenprüfung und Testpaket.".into(), file: "frame-000001.jpg".into() });
    meeting.analysis = Some(AnalysisResult {
        summary: vec![EvidenceItem {
            text: "Das Team prüft die Aufnahmequalität und bereitet ein Testpaket vor.".into(),
            sources: vec!["audio-0".into(), "audio-1".into()],
            ..Default::default()
        }],
        tasks: vec![EvidenceItem {
            text: "Audioaufnahme auf Windows prüfen.".into(),
            sources: vec!["audio-0".into()],
            owner: Some("Kim".into()),
            due_date: None,
        }],
        topics: vec![EvidenceItem {
            text: "Nächste Schritte aus der Präsentation".into(),
            sources: vec!["visual-0".into()],
            ..Default::default()
        }],
        ..Default::default()
    });
    meeting.markers.push(Marker {
        at_ms: 20000,
        text: "Nächste Schritte".into(),
    });
    for format in ["pdf", "md", "json", "srt", "vtt"] {
        exports::export(
            &meeting,
            &folder.join(format!("meetingpal-export.{format}")),
            format,
        )?;
    }
    println!("{}", folder.display());
    Ok(())
}
