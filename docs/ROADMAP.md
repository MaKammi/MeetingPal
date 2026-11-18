# Umsetzung und offene Produktarbeit

## 1. Repository und Grundgerüst

- [x] Privates Repository, `main`, Tauri 2 + React/TypeScript + Rust.
- [x] SQLite, lokale Dateien, Schlüsselverwaltung, Provider-Grenzen.
- [x] Automatische Prüfungen für Windows, Intel-/ARM-macOS und Linux; manuell gestartete Paket-Builds.

## 2. Aufnahme

- [x] Native Mikrofon-/Systemton-Pfade, getrennte Spuren, optionales Video und Zuschnitt.
- [x] Gerätewahl, Pegel, fünfsekündiger Audiotest, Pause, Hotkey, Markierungen und Notizen.
- [x] Hintergrundprozess, Segmente, Wiederherstellung, Import und Wiedergabe.
- [ ] Zwei Stunden auf jeder Zielplattform mit ≤100 ms Abweichung nachweisen.
- [ ] Verlustgrenze unter harten Prozess-/Systemabbrüchen messen; Segment-Commit-Latenz prüfen.
- [ ] Bluetooth, Hotplug, Ruhezustand, DPI-/Monitor-/Fensterwechsel auf echten Geräten abnehmen.
- [ ] Komfortable grafische Bereichsauswahl ergänzen; aktuell koordinatenbasierter Zuschnitt.

## 3. KI

- [x] Modellverwaltung mit festen Revisionen/Hashes; lokale Laufzeiten gebündelt.
- [x] Cloud-Anbieter, Offline-Prüfung, Aufrufbudget, fortsetzbare Aufträge.
- [x] Alle drei Betriebsarten, Meeting-Fragen, Bilder und native Gemini-Videoclips.
- [ ] Echte API-Abnahme für die vom Nutzer gewählten Modelle; Matrix modellgenauer Fähigkeiten erweitern.
- [x] Zweite ASR-Runde mit größerem Kontext und optionalem Turbo-Modell; Wiederaufnahme ohne doppelte Verarbeitung.
- [ ] Automatische Sprechertrennung über geeignete Modelle; gegenwärtig manuelle Labels.
- [ ] Lange native Videos über Anbieter-Datei-Uploads unterstützen; derzeit konservative Clipgrößengrenze.
- [ ] Optional Streaming auf Token-/Wortebene und parallele Priorisierung der KI-Aufträge.

## 4. Produktfertigstellung

- [x] Bibliothek, Suche, Bearbeitung, Belegsprünge, Exporte, versionierte Auswertungen.
- [x] Einwilligung, Übertragungsübersicht, lokale Speicherverwaltung, keine Telemetrie.
- [ ] Vollständige Offline-Netzwerkprüfung des installierten Produkts und Performance-Matrix.
- [ ] Signierte Windows-/macOS-Pakete, Notarisierung und vollständige Distributions-/Lizenzabnahme.
- [ ] Accessibility- und vollständiger Sprachreview; weitere Komfortfunktionen.

Team-Synchronisierung, Kalender, automatische Veröffentlichung und automatische Updates sind bewusst spätere Produktphasen.
