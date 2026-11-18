# Tests und Abnahme

## Automatisiert

`npm run check`: TypeScript, Produktions-Bundle und Frontend-Datenschutz-/Format-Tests.

`npm run test:e2e`: Browser-Interaktion für leere Bibliothek, Modellverwaltung, Aufnahme-Dialog, Einwilligung, Cloud-/Offline-Umschaltung und Einstellungen. Die Browser-Vorschau ruft keine echten Aufnahmefunktionen auf. Dabei wird geprüft, dass die Seite keine externen Assets lädt.

`cargo test --manifest-path src-tauri/Cargo.toml --locked`: Offline-/Budgetgrenzen, Quellenprüfung, Ausschnittschutz, Originalaudio-Unverändertheit, dauerhafte/idempotente Aufträge, Wiederherstellung, Pfadgrenzen, PDF und mehrstündige Untertitelzeiten.

CI baut die Rust-Anwendung zusätzlich für Windows x64, macOS ARM64/x64 und Linux x64. Erfolgreiches Kompilieren beweist keine Mikrofon-, Portal-, Bluetooth- oder Echtzeiteigenschaften.

## Hardware-Abnahme – vor Produktfreigabe offen

Pro Gerät OS-Version, CPU/GPU, RAM, Audiogeräte, Abtastraten, Display-Skalierung, Modelle und Laufzeitstände protokollieren. Ausschließlich freigegebenes Testmaterial verwenden.

| Test | Erfolgskriterium |
|---|---|
| Zwei Stunden, Mic + Desktop, Video an/aus | Keine verlorenen abgeschlossenen Segmente; gemessene A/V- und Spurabweichung ≤100 ms |
| Aufnahmeprozess beenden, App neu starten | Abgeschlossene Segmente wieder da; höchstens letztes Fünfsekundensegment fehlt |
| Pause mehrfach, einschließlich kurz vor Segmentgrenze | Keine Pausen-Audiodaten im aktiven Verlauf; Video-/Audiozeit bleibt gemeinsam |
| Mikrofon-/Screen-Rechte verweigern | Sichtbare Erklärung, kein falscher Aufnahme-Erfolg |
| USB entfernen, Bluetooth wechseln, Standardgerät ändern | Fehler/Lücken sichtbar; sinnvolle Wiederherstellung ohne Audioverlust anderer Spur |
| Fenster schließen/vergrößern, Monitore/DPI wechseln | Nur gewählte Pixel; bei ungültigem Bereich Abbruch des Bildpfads, Audio läuft weiter |
| Disk voll / Standby / Bildschirm gesperrt | Erhaltene Segmente, klare Warnungen, kein stiller Erfolg |
| Live/Assistant mit langsamer CPU | Rückstand sichtbar; Aufnahme bleibt unabhängig |
| Netz aus, falscher Schlüssel, 429, Budget erschöpft | KI stoppt/wartet nachvollziehbar; Aufnahme ungestört; Wiederholung ohne doppelte Ergebnisse |
| Deutsch/Englisch, Überlappung, Stille, Fachbegriffe | Keine erfundenen Verantwortlichen/Termine; Belege passen; Qualität manuell dokumentiert |
| Präsentation, Frames und native Clips | Crop bleibt erhalten; Zeitmarken und Quellen stimmen |
| PDF/Markdown/JSON/SRT/VTT | Umlaute, lange Texte, Stundenzeiten, Sprecher und Belege stimmen |
| Offline-Profil nach Modellinstallation | Firewall/Packet-Capture: keine externen Verbindungen aus App und Unterprozessen; Loopback zulässig |
| Installer/Deinstallation auf sauberem Gerät | App und gebündelte Laufzeiten starten; persönliche Bibliothek bleibt kontrollierbar |

Zielgeräte: Windows 11 x64; macOS 14.2+ Intel und Apple Silicon; Ubuntu 24.04 GNOME unter Wayland **und** X11. CI-Mac-Runners verwenden ein neueres macOS und ersetzen insbesondere den 14.2-Test nicht.

## Freigabe

- Signierungszertifikate/Notarisierung sicher in CI-Secrets konfigurieren; diese werden nicht im Repository abgelegt.
- FFmpeg-/Codec-Konfiguration sowie sämtliche Lizenz- und korrespondierenden Quellmaterialien prüfen.
- Modelldateien bleiben getrennte, ausdrückliche Downloads; Lizenzdarstellung prüfen.
- Abnahmematrix mit Messwerten ausfüllen; keine ungeprüften Zielwerte als Garantien veröffentlichen.
- Testpakete sind private Actions-Artefakte, keine öffentliche Veröffentlichung oder automatische Updates.
