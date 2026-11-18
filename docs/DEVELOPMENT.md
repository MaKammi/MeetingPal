# Entwicklung

## Struktur

| Pfad | Aufgabe |
|---|---|
| `src/` | React-Oberfläche, deutsch/englische Texte, Typen und IPC-Client |
| `src-tauri/src/capture*.rs` | Native Aufnahme, Segmentierung und Synchronisierung |
| `src-tauri/src/storage.rs` | Bibliothek, Suche, Wiederherstellung, Aufträge |
| `src-tauri/src/models.rs` | Modellkatalog und geprüfte Downloads |
| `src-tauri/src/providers.rs` | Cloud- und lokale KI-Adapter |
| `src-tauri/src/jobs.rs` | Fortsetzbare Verarbeitung |
| `src-tauri/src/media.rs` | Import, Medienkonvertierung, Wiedergabe, Bilder |
| `src-tauri/src/exports.rs` | Markdown, PDF, JSON, Untertitel |
| `resources/` | Gepinnte Kataloge, Schrift/Lizenz, Testaudio, Icon |
| `scripts/` | Runtime-Builds und explizite Aktualisierung des Katalogs |
| `third-party/` | Gepatchter scap-Quellcode samt Lizenz |

Die Browser-Vorschau zeigt absichtlich eine leere Bibliothek und kann keine Desktop-Operationen vortäuschen. Für Aufnahme-/Importtests `npm run desktop` verwenden.

## Laufzeiten

`npm run runtimes:build` erstellt `src-tauri/binaries`. Teilbuilds sind mit `node scripts/build-runtimes.mjs --only=whisper,llama` möglich. Downloads und Builds liegen im ignorierten `vendor/`-Verzeichnis. Runtimes werden aus `resources/runtimes.json` geladen; KI-Quellen sind auf Commit-IDs festgelegt. FFmpeg-Archive werden vor Entpacken mit SHA-256 geprüft.

Die Standardbuilds verwenden CPU-Verarbeitung auf Windows/Linux und Metal-Unterstützung auf macOS, sofern vom jeweiligen Upstream aktiviert. Kein CUDA-/Vulkan-Zwang. Lokale Echtzeitfähigkeit muss mit dem Leistungscheck und der tatsächlichen Hardware bewertet werden. Windows-CPU-Binaries benötigen AVX2; eine ältere CPU-Baseline ist ein separates Distributionsziel.

`npm run catalog:update` aktualisiert den Katalog ausdrücklich anhand der Hugging-Face-LFS-Metadaten. Änderungen an Revision, Dateigröße, Lizenz und SHA-256 vor Commit prüfen. Das installierte Programm lädt keine Modelle selbstständig.

## Anbieter

Die Modell-ID wird vom Nutzer ausgewählt oder aus dem authentifizierten Modellkatalog des Anbieters übernommen. Es gibt keinen unsichtbaren Anbieterwechsel. Geheimnisse werden nur im Rust-Prozess verarbeitet; die Oberfläche erhält ausschließlich den Status „vorhanden“ und speichert keine Schlüssel in Einstellungen.

Provider-Antworten sind Daten. Keine Tools, Funktionsaufrufe, Shell-Kommandos oder externen Aktionen aus Meeting-Inhalten. Systemanweisungen verlangen quellenbasierte JSON-Ausgaben; alle Belege müssen im Meeting existieren. Die App kann die inhaltliche Richtigkeit einer gültigen Quellenreferenz nicht automatisch garantieren.

Ein neuer Adapter muss Aufgaben-/Eingabefähigkeiten melden, Offline- und Budgetprüfung verwenden, Timeouts behandeln und Ergebnisse in die internen Schemas abbilden. Modellfähigkeiten unterscheiden sich innerhalb eines Anbieters; unbekannte Modellkombinationen dürfen nicht als bereits getestet dargestellt werden.

## Betriebssysteme

Linux-Pakete siehe `.github/workflows/ci.yml`. Auf Wayland ist eine aktive Portal-/PipeWire-Sitzung notwendig; Headless-CI ersetzt keinen Aufnahmeversuch. Auf macOS sind Mikrofon- und Bildschirmfreigabe nötig. Nicht signierte Testbuilds sind nicht für reguläre öffentliche Verteilung vorgesehen.

Echte Provider-Tests sind opt-in und benötigen eigene Schlüssel. Schlüssel, Meeting-Inhalte, Exporte, Modelldateien und Debug-Logs nicht in Issues oder Commits aufnehmen. Es ist kein Auto-Update- oder Telemetrie-Dienst eingebaut.
