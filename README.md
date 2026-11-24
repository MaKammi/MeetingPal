# MeetingPal

Desktop-App für lokale Meeting-Aufnahmen und KI-gestützte Protokolle. MeetingPals eigener Quellcode steht unter der [GNU Affero General Public License, Version 3 (AGPL-3.0-only)](LICENSE). Für Beiträge gilt das [Contributor License Agreement](CONTRIBUTING.md).

**[MeetingPal für Windows, macOS und Linux herunterladen](https://github.com/MaKammi/MeetingPal/releases/latest)**. Die erste öffentliche Version enthält native Aufnahme und KI-Auswertung. Die Abnahme auf echten Geräten läuft noch. Dazu gehören die Messungen zur Synchronität und zur Wiederherstellung nach einem Abbruch. [Abnahmestand](docs/TESTING.md)

## Funktionsumfang

- Mikrofon und Desktop-Ton als separate WAV-Spuren; zusätzlich ein abgeleiteter Mix für die KI.
- Optional Bildschirm, Fenster oder zugeschnittener Bereich; segmentiertes WebM mit VP8, Wiedergabe mit Opus-Ton.
- Nachträgliche Verarbeitung, abschnittsweises Live-Transkript und Live-Assistenz. Die Aufnahme läuft in einem eigenen Prozess unabhängig von der KI.
- Lokale Bibliothek, Tags, Notizen, Zeitmarken, Transkriptbearbeitung und manuelle Sprechernamen.
- Zusammenfassung, Themen, Entscheidungen, Aufgaben und offene Fragen mit überprüfbaren Quellenverweisen; Fragen zum Meeting.
- OpenAI für Audio; OpenAI, Anthropic und Gemini für Text/Bilder. Gemini zusätzlich für native Videoclips. Die Modell-ID wird pro Aufgabe gewählt.
- Integrierter Downloadmanager für Whisper Small/Turbo und Qwen3-Text-/Vision-Modelle; Größen, Lizenzen, SHA-256-Prüfung, Abbruch und Löschung. Lokale Verarbeitung mit gebündeltem whisper.cpp/llama.cpp.
- Exporte: Markdown, PDF mit eingebetteter Schrift, JSON, SRT und VTT.
- Schlüssel im Betriebssystem-Schlüsselspeicher; Sitzungs-Fallback. Keine zusätzliche Bibliotheksverschlüsselung und keine Telemetrie.

Live verarbeitet abgeschlossene Audioabschnitte. Unterstützte OpenAI-Modelle zeigen auf Wunsch bereits während der Verarbeitung Text an. `gpt-4o-transcribe-diarize` liefert Sprecher und Zeitmarken. Die Sprecherkennungen gelten für den jeweiligen Audioabschnitt und lassen sich nachträglich bearbeiten. In der Modellauswahl lassen sich Cloud-Aufgaben mit öffentlichem Testaudio oder synthetischen Inhalten prüfen. Dabei können API-Kosten entstehen.

## Start aus dem Quellcode

Voraussetzungen: Node.js 24, aktuelle stabile Rust-Toolchain, CMake, Git und die [Tauri-Systemabhängigkeiten](https://v2.tauri.app/start/prerequisites/). Windows benötigt die Visual-C++-Buildtools, macOS Xcode Command Line Tools; Linux die in der CI aufgeführten Entwicklungsbibliotheken.

```sh
npm ci
npm run runtimes:build
npm run desktop
```

Der Runtime-Build lädt festgelegte Quellstände beziehungsweise geprüfte Archive und kompiliert lokale KI-Laufzeiten. Modelle sind **nicht** enthalten: Unter **KI-Modelle** ausdrücklich installieren. Alternativ unter **Einstellungen** eigene API-Schlüssel hinterlegen und im Aufnahme-Dialog die gewünschten Cloud-Modelle auswählen. Das lokale Profil wechselt niemals automatisch zur Cloud.

```sh
npm run check
npm run test:e2e
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run desktop:build
```

Vor dem ersten Browser-Test: `npx playwright install chromium`. `npm run dev` startet nur eine Browser-Vorschau, keine browserbasierte Aufnahme.

## Benutzung

1. Meeting aufnehmen oder vorhandene Mediendatei importieren.
2. Titel, Betriebsart, Mikrofon, Desktop-Ton und optional Videoquelle auswählen. Zuschnittkoordinaten beziehen sich auf Pixel der Quelle.
3. Modelle, Sprache, Übertragungen und Anfragenbudget prüfen; notwendige Einwilligung bestätigen. Ein fünfsekündiger Audiotest prüft Geräte und Wiedergabe.
4. Aufnahme starten; Pause/Fortsetzen auch mit **Strg+Umschalt+Leertaste** bzw. **Cmd+Umschalt+Leertaste**. Fehler und fehlende Audiodaten werden angezeigt.
5. Ergebnisse prüfen, Transkript/Sprecher korrigieren, Auswertung erneut erstellen und exportieren. KI-Ausgaben sind Entwürfe.

Bibliothek und Modelle liegen im App-Datenverzeichnis `de.meetingpal.desktop`; der genaue Pfad steht in den Einstellungen. Für eine konsistente Sicherung die App beenden und das gesamte Verzeichnis kopieren. API-Schlüssel sind nicht Teil dieser Sicherung. Aufbewahrungsregeln sind standardmäßig aus.

## Plattformen und Pakete

Entwicklungsziele: Windows 11 x64; macOS ab 14.2 auf Intel/Apple Silicon; Ubuntu 24.04 x64 mit GNOME auf Wayland/X11. Unter Wayland entscheidet die Systemfreigabe über Bildschirm/Fenster. Auf macOS benötigt auch Desktop-Ton die ScreenCaptureKit-Freigabe; Systemton umfasst alle Apps.

Windows verwendet den x64-Installer, macOS eine `.dmg` für Apple Silicon (`aarch64`) oder Intel (`x64`), Ubuntu ein `.deb` oder eine `.AppImage`. Nutzer von 0.1.0 installieren die öffentliche Version einmal manuell. Die lokale Bibliothek wird weiterverwendet.

Ab 0.1.1 prüft die App beim Start und bei längerem Betrieb täglich auf Updates, wenn das Standardprofil Onlinezugriff erlaubt. Unter **Einstellungen → Programmupdates** lässt sich die Suche auch bei lokaler KI erlauben oder auf manuell stellen. **Jetzt nach Updates suchen** verbindet sich ausdrücklich mit GitHub. Nach Bestätigung lädt die App das passende signierte Paket und startet neu. Während einer Aufnahme oder laufenden Verarbeitung lässt sich kein Update installieren.

Gewöhnliche Pushes und Pull Requests verwenden einen Linux-Runner. Ein Versionstag baut und testet alle vier Plattformziele, prüft die Signaturen und veröffentlicht Installer und Updatefeed gemeinsam. Der vollständige Plattformtest lässt sich auch manuell starten. Die Pakete haben eine Updatersignatur. Windows-Herausgebersignatur und macOS-Notarisierung stehen aus. [Releaseablauf](docs/DEVELOPMENT.md#veröffentlichung)

## Dokumentation

- [Architektur und Datenfluss](docs/ARCHITECTURE.md)
- [Entwicklung und Provider](docs/DEVELOPMENT.md)
- [Datenschutz](docs/PRIVACY.md)
- [Aufgaben und Produktfreigabe](docs/ROADMAP.md)
- [Tests und offene Abnahmen](docs/TESTING.md)
- [Drittanbieter und Weitergabe](docs/THIRD_PARTY.md)
