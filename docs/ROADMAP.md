# Umsetzung und offene Produktarbeit

## 1. Repository und Grundgerüst

- [x] Öffentliches Repository mit AGPL-3.0-only, `main`, Tauri 2 + React/TypeScript + Rust.
- [x] SQLite, lokale Dateien, Schlüsselverwaltung, Provider-Grenzen.
- [x] Automatische Linux-Prüfung bei Änderungen, Paket- und Medienprüfungen für alle vier Ziele beim Release.

## 2. Aufnahme

- [x] Native Mikrofon-/Systemton-Pfade, getrennte Spuren, optionales Video und Zuschnitt.
- [x] Gerätewahl, Pegel, fünfsekündiger Audiotest, Pause, Hotkey, Markierungen und Notizen.
- [x] Hintergrundprozess, Segmente, Wiederherstellung, Import und Wiedergabe.
- [ ] Zwei Stunden auf jeder Zielplattform mit ≤100 ms Abweichung nachweisen.
- [ ] Verlustgrenze bei System-/Stromausfall auf Zielgeräten messen. Der automatisierte Prozessabbruch prüft echte Segmentdateien, Wiederherstellung und Commit-Latenz.
- [ ] Bluetooth, Hotplug, Ruhezustand, DPI-/Monitor-/Fensterwechsel auf echten Geräten abnehmen.
- [x] Grafische Bereichsauswahl für Bildschirmaufnahmen mit Vorschau, Maus und Tastatur.

## 3. KI

- [x] Modellverwaltung mit festen Revisionen/Hashes; lokale Laufzeiten gebündelt.
- [x] Cloud-Anbieter, Offline-Prüfung, Aufrufbudget, fortsetzbare Aufträge.
- [x] Alle drei Betriebsarten, Meeting-Fragen, Bilder und native Gemini-Videoclips.
- [x] Modellgenaue Fähigkeiten im Auswahldialog und echte, ausdrücklich ausgelöste API-Tests mit festen Testdaten. Die am 24.11.2026 geprüften Modell-IDs stehen im Testprotokoll.
- [x] Zweite ASR-Runde mit größerem Kontext und optionalem Turbo-Modell; Wiederaufnahme ohne doppelte Verarbeitung.
- [x] Automatische Sprechertrennung mit OpenAI-Diarisierung, Abschnittszeitmarken und weiterhin editierbaren Namen.
- [x] Lange Videos abschnittsweise über die Gemini Files API übertragen, Verarbeitung abwarten und hochgeladene Dateien löschen.
- [x] Optionale Streaming-Vorschau während der OpenAI-Transkription und zwei parallel arbeitende, priorisierte KI-Worker für unabhängige Meetings.

## 4. Produktfertigstellung

- [x] Bibliothek, Suche, Bearbeitung, Belegsprünge, Exporte, versionierte Auswertungen.
- [x] Einwilligung, Übertragungsübersicht, lokale Speicherverwaltung, keine Telemetrie.
- [ ] Offline- und Leistungsmessungen an den installierten Zielpaketen vervollständigen. Der Pakettest misst Import, Wiedergabe und PDF-Export. Mit bereits installierten Modellen prüft er zusätzlich die lokale KI.
- [ ] Windows-Herausgebersignatur und macOS-Signatur/Notarisierung sind zurückgestellt.
- [ ] Distributionsmaterial in den veröffentlichten Paketen prüfen. Der Pakettest gleicht die fünf Quellarchive, die Lizenzen, die Codec-Konfiguration und die AppImage-Bibliotheken mit den Releasequellen ab.
- [x] Deutsche und englische Oberfläche einschließlich Auftragsstatus und integrierter Fehlermeldungen durchgesehen. Bibliothek, Einstellungen, Modellkatalog, Aufnahme, Quellenauswahl, Zuschnitt und alle fünf Meeting-Detailansichten mit Tastatur und automatisierten WCAG-Prüfungen getestet.
- [x] Sprachausgabe von Orca für Bibliothek, Suche, Aufnahmedialog und Fokusrückgabe mit dem installierten Ubuntu-Paket auf Deutsch und Englisch geprüft.
- [ ] Manueller Nutzungstest mit Screenreader auf den Zielgeräten.
- [x] Öffentliche Releases, signierte Programmupdates und manueller oder automatischer Updatecheck mit Offline-Einstellung.

Team-Synchronisierung und Kalenderintegration sind spätere Produktphasen.
