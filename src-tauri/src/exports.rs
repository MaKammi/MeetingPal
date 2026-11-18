use crate::domain::*;
use anyhow::Result;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, Stream,
};
use std::{collections::BTreeMap, path::Path};

pub fn timestamp(ms: u64, separator: char) -> String {
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}
pub fn subtitles(meeting: &Meeting, vtt: bool) -> String {
    let mut out = if vtt {
        "WEBVTT\n\n".into()
    } else {
        String::new()
    };
    for (i, s) in meeting
        .transcript
        .iter()
        .filter(|s| s.end_ms > s.start_ms && !s.text.trim().is_empty())
        .enumerate()
    {
        let separator = if vtt { '.' } else { ',' };
        let text = s.text.replace("-->", "→").replace(['\r', '\n'], " ");
        out.push_str(&format!(
            "{}\n{} --> {}\n{}{}\n\n",
            i + 1,
            timestamp(s.start_ms, separator),
            timestamp(s.end_ms, separator),
            s.speaker
                .as_ref()
                .map(|s| format!("{s}: "))
                .unwrap_or_default(),
            text
        ));
    }
    out
}
pub fn markdown(meeting: &Meeting) -> String {
    report(meeting, true)
}
fn anchor(id: &str) -> String {
    format!("source-{}", hex::encode(id.as_bytes()))
}
fn source_label(meeting: &Meeting, id: &str) -> String {
    if let Some(s) = meeting.transcript.iter().find(|s| s.id == id) {
        timestamp(s.start_ms, '.')
    } else if let Some(v) = meeting.visuals.iter().find(|v| v.id == id) {
        format!("Video {}", timestamp(v.at_ms, '.'))
    } else {
        id.to_owned()
    }
}
fn report(meeting: &Meeting, links: bool) -> String {
    let mut out = format!(
        "# {}\n\n{} · {}\n\n",
        meeting.config.title,
        meeting.created_at,
        timestamp(meeting.duration_ms, '.')
    );
    if let Some(a) = &meeting.analysis {
        for (title, items) in [
            ("Summary", &a.summary),
            ("Topics", &a.topics),
            ("Decisions", &a.decisions),
            ("Tasks", &a.tasks),
            ("Open questions", &a.questions),
        ] {
            out.push_str(&format!("## {title}\n\n"));
            for item in items {
                out.push_str(&format!(
                    "- {}{}{} ({})\n",
                    item.text,
                    item.owner
                        .as_ref()
                        .map(|s| format!(" — {s}"))
                        .unwrap_or_default(),
                    item.due_date
                        .as_ref()
                        .map(|s| format!(" · {s}"))
                        .unwrap_or_default(),
                    item.sources
                        .iter()
                        .map(|id| {
                            let label = source_label(meeting, id);
                            if links {
                                format!("[{label}](#{})", anchor(id))
                            } else {
                                format!("{label}; {id}")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            out.push('\n');
        }
    }
    out.push_str(&format!(
        "## Notes\n\n{}\n\n## Transcript\n\n",
        meeting.notes
    ));
    for s in &meeting.transcript {
        if links {
            out.push_str(&format!("<a id=\"{}\"></a>\n\n", anchor(&s.id)));
        }
        out.push_str(&format!(
            "### {} · {}\n\n{}\n\nSource: {}\n\n",
            timestamp(s.start_ms, '.'),
            s.speaker.as_deref().unwrap_or("Speaker"),
            s.text,
            s.id
        ));
    }
    if !meeting.visuals.is_empty() {
        out.push_str("## Visual evidence\n\n");
        for v in &meeting.visuals {
            if links {
                out.push_str(&format!("<a id=\"{}\"></a>\n\n", anchor(&v.id)));
            }
            out.push_str(&format!(
                "### Video {}\n\n{}\n\nSource: {}\n\n",
                timestamp(v.at_ms, '.'),
                v.text,
                v.id
            ));
        }
    }
    if !meeting.markers.is_empty() {
        out.push_str("## Bookmarks\n\n");
        for marker in &meeting.markers {
            out.push_str(&format!(
                "- {} — {}\n",
                timestamp(marker.at_ms, '.'),
                marker.text
            ));
        }
    }
    out
}
pub fn export(meeting: &Meeting, path: &Path, format: &str) -> Result<()> {
    match format {
        "json" => std::fs::write(path, serde_json::to_vec_pretty(meeting)?)?,
        "md" => std::fs::write(path, markdown(meeting))?,
        "srt" | "vtt" => std::fs::write(path, subtitles(meeting, format == "vtt"))?,
        "pdf" => pdf(meeting, path)?,
        _ => anyhow::bail!("Unsupported export format"),
    }
    Ok(())
}
fn pdf(meeting: &Meeting, path: &Path) -> Result<()> {
    let bytes = include_bytes!("../../resources/NotoSans-Regular.ttf");
    let face =
        ttf_parser::Face::parse(bytes, 0).map_err(|_| anyhow::anyhow!("Invalid PDF font"))?;
    let text = report(meeting, false);
    let mut chars = BTreeMap::<u16, char>::new();
    let mut lines = vec![];
    for original in text.lines().map(|s| s.trim_start_matches('#').trim()) {
        let mut line = String::new();
        let mut width = 0.0f32;
        for word in original.split_whitespace() {
            let w: f32 = word
                .chars()
                .map(|c| {
                    face.glyph_index(c)
                        .and_then(|g| face.glyph_hor_advance(g))
                        .unwrap_or(500) as f32
                        / face.units_per_em() as f32
                        * 10.0
                })
                .sum::<f32>()
                + 3.0;
            if w > 490.0 {
                if !line.is_empty() {
                    lines.push(line);
                    line = String::new();
                    width = 0.0;
                }
                for c in word.chars() {
                    let advance = face
                        .glyph_index(c)
                        .and_then(|g| face.glyph_hor_advance(g))
                        .unwrap_or(500) as f32
                        / face.units_per_em() as f32
                        * 10.0;
                    if width + advance > 490.0 && !line.is_empty() {
                        lines.push(line);
                        line = String::new();
                        width = 0.0;
                    }
                    line.push(c);
                    width += advance;
                }
                continue;
            }
            if width + w > 490.0 && !line.is_empty() {
                lines.push(line);
                line = String::new();
                width = 0.0;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
            width += w;
        }
        lines.push(line);
    }
    for line in &lines {
        for c in line.chars() {
            if let Some(g) = face.glyph_index(c) {
                chars.entry(g.0).or_insert(c);
            }
        }
    }
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font_file = doc.add_object(Stream::new(
        dictionary! {"Length1"=>bytes.len() as i64},
        bytes.to_vec(),
    ));
    let descriptor=doc.add_object(dictionary!{"Type"=>"FontDescriptor","FontName"=>"NotoSans","Flags"=>32,"FontBBox"=>vec![(-700).into(),(-400).into(),(2800).into(),(1200).into()],"ItalicAngle"=>0,"Ascent"=>1069,"Descent"=>-293,"CapHeight"=>714,"StemV"=>80,"FontFile2"=>font_file});
    let mut widths = vec![];
    for id in chars.keys() {
        widths.push(Object::Integer(*id as i64));
        widths.push(Object::Array(vec![Object::Integer(
            face.glyph_hor_advance(ttf_parser::GlyphId(*id))
                .unwrap_or(500) as i64
                * 1000
                / face.units_per_em() as i64,
        )]));
    }
    let cid=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"CIDFontType2","BaseFont"=>"NotoSans","CIDSystemInfo"=>dictionary!{"Registry"=>Object::string_literal("Adobe"),"Ordering"=>Object::string_literal("Identity"),"Supplement"=>0},"FontDescriptor"=>descriptor,"DW"=>1000,"W"=>widths,"CIDToGIDMap"=>"Identity"});
    let mut cmap=String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    let mappings: Vec<_> = chars.iter().collect();
    for chunk in mappings.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (id, c) in chunk {
            let mut units = [0u16; 2];
            let encoded = c.encode_utf16(&mut units);
            cmap.push_str(&format!(
                "<{id:04X}> <{}>\n",
                encoded
                    .iter()
                    .map(|v| format!("{v:04X}"))
                    .collect::<String>()
            ));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    let unicode = doc.add_object(Stream::new(dictionary! {}, cmap.into_bytes()));
    let font=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type0","BaseFont"=>"NotoSans","Encoding"=>"Identity-H","DescendantFonts"=>vec![Object::Reference(cid)],"ToUnicode"=>unicode});
    let mut kids = vec![];
    for (page_index, chunk) in lines.chunks(48).enumerate() {
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), 10.into()]),
            Operation::new("Td", vec![50.into(), 790.into()]),
            Operation::new("TL", vec![15.into()]),
        ];
        for line in chunk {
            let glyphs: Vec<u8> = line
                .chars()
                .flat_map(|c| {
                    face.glyph_index(c)
                        .unwrap_or(ttf_parser::GlyphId(0))
                        .0
                        .to_be_bytes()
                })
                .collect();
            operations.push(Operation::new(
                "Tj",
                vec![Object::String(glyphs, lopdf::StringFormat::Hexadecimal)],
            ));
            operations.push(Operation::new("T*", vec![]));
        }
        operations.push(Operation::new("ET", vec![]));
        let content = doc.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode()?,
        ));
        let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Contents"=>content,"Tabs"=>"S"});
        kids.push(Object::Reference(page));
        let _ = page_index;
    }
    let count = kids.len() as i64;
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count}),
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    doc.compress();
    doc.save(path)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subtitle_timestamps_cover_long_meetings() {
        assert_eq!(timestamp(3_661_042, ','), "01:01:01,042");
    }
    #[test]
    fn exports_include_resolvable_visual_sources() {
        let mut m = Meeting::new(MeetingConfig::default());
        m.visuals.push(VisualEvidence {
            id: "frame-1".into(),
            at_ms: 15000,
            text: "Slide: next release in December".into(),
            file: "frame-1.jpg".into(),
        });
        m.analysis = Some(AnalysisResult {
            summary: vec![EvidenceItem {
                text: "Release planned".into(),
                sources: vec!["frame-1".into()],
                ..Default::default()
            }],
            ..Default::default()
        });
        let markdown = markdown(&m);
        assert!(markdown.contains(&format!("(#{}", anchor("frame-1"))));
        assert!(markdown.contains(&format!("id=\"{}\"", anchor("frame-1"))));
        assert!(markdown.contains("Slide: next release in December"));
        assert!(!report(&m, false).contains("<a id="));
    }
    #[test]
    fn pdf_is_valid_and_paginates() {
        let mut m = Meeting::new(MeetingConfig::default());
        m.config.title = "Prüfung – Übergabe".into();
        m.notes = "Überblick zur Qualität\n".repeat(120);
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("test.pdf");
        pdf(&m, &p).unwrap();
        let doc = Document::load(p).unwrap();
        assert!(doc.get_pages().len() >= 3);
    }
}
