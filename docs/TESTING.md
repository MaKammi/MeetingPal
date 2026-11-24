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
- Der Paketlauf prüft die Medienpipeline mit den tatsächlich gebündelten Codecs im Release-Profil auf vier Zielen. Manuelle Testpakete bleiben zwei Tage als Actions-Artefakte verfügbar.
- Bei einem Versionstag prüft der Releasejob die Manifestdaten jeder Plattform, verifiziert die Updatepakete mit dem öffentlichen Schlüssel und überprüft die SHA-256-Werte im GitHub-Entwurf. Nutzer erreichen die Installer und den Updatefeed über das öffentliche Release.
- Nach der Veröffentlichung prüft `cargo test --manifest-path src-tauri/Cargo.toml --test published_update -- --ignored --nocapture` den anonymen öffentlichen Feed mit dem echten Tauri-Updater. Eine simulierte ältere Version lädt das signierte Plattformpaket herunter, die aktuelle Version meldet keinen Updatebedarf. Der Test führt keinen Installer aus.

## Nachweise für 0.1.2 vom 24. November 2026

Die Windows-Entwicklungsmaschine lief mit Windows 11 Pro (Build 26300) auf einem Intel Core i7-1265U. Der erzwungene Abbruch eines separaten Testprozesses ließ drei vollständige Fünfsekundensegmente einschließlich beider Originalspuren und Audiomix wiederherstellen. Der teilweise geschriebene vierte Abschnitt gelangte nicht ins Manifest. Die drei Commit-Zeiten im Debug-Lauf betrugen 52,5, 61,6 und 53,6 ms. Ein Stromausfall und die Capture-Hardware sind eigene Prüffälle.

Die Cloud-Abnahme verwendet dieselben Adapter wie die Modellauswahl. Für die folgenden Stichproben waren die Zugangsdaten in MeetingPal hinterlegt.

| Anbieter und Modell | Durchgeführter Aufruf | Ergebnis |
|---|---|---|
| OpenAI `gpt-4o-transcribe-diarize` | Öffentliches Testaudio, fortlaufende Sprechersegmente und Zeitmarken | Bestanden |
| OpenAI `gpt-4o-mini-transcribe` | Dasselbe Audio mit Text-Deltas und abgeschlossenem Transkript | Bestanden |
| OpenAI `gpt-4.1-mini` und Anthropic `claude-haiku-4-5` | Strukturierte Auswertung eines synthetischen Transkripts mit Beleg-ID | Beide bestanden |
| Gemini `gemini-2.5-flash` | Synthetisches WebM hochladen, verarbeiten, beschreiben und löschen | Bestanden |

Die lokalen UI-Tests prüfen die Tastaturbedienung der verschachtelten Dialoge, die Fokusrückgabe, die deutsch- und englischsprachige Modellauswahl und die Bibliothekssuche. Die automatisierte WCAG-Prüfung (axe-core) findet in Bibliothek, Aufnahme-Dialog, Modellkatalog und Einstellungen keine Verstöße der geprüften Stufen A/AA. Manuelle Prüfungen mit Screenreadern und auf weiteren Plattformen bleiben Teil der Abnahme.

Der Befehl `cargo test --manifest-path src-tauri/Cargo.toml --test cloud_models -- --ignored --nocapture --test-threads=1` wiederholt die kostenpflichtigen API-Tests. Ein anderes Modell lässt sich direkt im Auswahldialog mit Testdaten prüfen.

### Synchronität auf Geräten messen

Für die Messung eine ruhige Testumgebung mit kontrolliertem Mikrofoneingang verwenden. Alle 60 Sekunden ein gemeinsames sichtbares und hörbares Referenzsignal abspielen, beispielsweise einen weißen Vollbildblitz auf Schwarz und einen gleichzeitigen 250-ms-Ton. Das Signal muss auf beiden Audiospuren erscheinen. Die Aufnahme vor dem ersten Signal starten, mindestens zwei Stunden laufen lassen und die Wiedergabe vorbereiten. Die Zeitauflösung der Auswertung beträgt 10 ms für Audio und rund 17 ms für Video. Das Skript prüft Dauer, durchgängige Segmente, Markeranzahl und die maximale paarweise Abweichung.

```sh
python scripts/measure-capture.py MEETINGORDNER --device "OS, Hardware, Skalierung, Eingabe und Ausgabe" --ffmpeg PFAD_ZU_FFMPEG
```

Für die Audio-Abnahme dient zusätzlich `--audio-only`. `python scripts/test-measure-capture.py` kontrolliert die Messung mit synchronen, absichtlich verspäteten und fehlenden Signalen. Der Rust-Test für zwei simulierte Stunden verwendet unabhängige Geräteuhren mit ±200 ppm und 0–3 ms Zustellschwankung. Im Windows-Test lag die berechnete obere Grenze der Spurabweichung bei 38,00 ms.

### Veröffentlichte Pakete prüfen

Der GitHub-Workflow **Installed release acceptance** lädt die Installer des angegebenen Tags, vergleicht SHA-256, prüft mitgelieferte Lizenz- und Quelldateien und startet die installierte Anwendung in einer temporären Bibliothek. Die vorab verifizierten Whisper- und Qwen-Modelle bleiben beim Netzwerktest lokal. Der Test umfasst den WebView-Start, Import des öffentlichen Benchmark-Audios, Wiedergabeaufbereitung, echte lokale Inferenz und PDF-Export. Für jedes Ziel entsteht ein JSON-Bericht mit Laufzeiten. Unter Linux beobachtet `strace` auch die Kindprozesse, unter Windows die Filtering Platform zusammen mit der Prozessüberwachung. macOS beobachtet die IP-Pakete der Anwendung, der lokalen Laufzeiten und der WebKit-Prozesse mit PKTAP und prüft den Loopback-Verkehr der lokalen Inferenz als Positivkontrolle. Auf einem eigenen Rechner führt `--check-offline NEUE_BERICHTSDATEI [MODELLORDNER]` denselben Produktdurchlauf aus. Ohne Modellordner entfallen die Inferenzmessungen.

## Zusätzliche Prüfungen für 0.1.3

Die automatisierte Prüfung mit axe-core erstreckt sich auch auf Quellenauswahl, Vorschau, Zuschnitt und die fünf Detailansichten eines bereits verarbeiteten Meetings. Für die Detailansichten prüft Playwright Deutsch und Englisch mit Beispieldaten, einschließlich leerer Ergebnislisten und eines abgeschlossenen Auftrags. Testfälle prüfen die Übersetzung gespeicherter Warnungen sowie den Erhalt von Modell-IDs, HTTP-Fehlercodes und externen Systemmeldungen. Der lokale Whisper-/Qwen-Integrationstest mit dem öffentlichen Audio einschließlich manueller Korrektur und anschließender Meeting-Frage bestand erneut.

Am 24. November 2026 lieferten die Anbieterlisten 65 Gemini-, 99 OpenAI- und 14 Anthropic-Einträge. Nach Auswahl für den jeweiligen Aufgabentyp bestanden `gemini-4-argon` (Videoclip), `gpt-6.2-sol` und `claude-fable-5-2` (belegte Zusammenfassung) sowie `gpt-transcribe` (öffentliches Audio mit Streaming) den echten API-Test. Beim nächsten Abruf entscheidet der dann aktuelle Anbieterkatalog über die Vorschläge. Playwright prüft zusätzlich den Wechsel aus dem lokalen Meeting zu Gemini, Abbrechen, gespeicherte Modell-IDs, aktualisierte Modelllisten und verspätete Antworten anderer Anbieter.

Ein synthetisches Video von 123 Sekunden durchlief die echte Gemini-Videopipeline einschließlich Upload, Verarbeitung und Löschung. Die drei übermittelten Clips waren 37.538.907, 38.076.853 und 1.903.975 Byte groß. Für diesen Test fielen 18 Anbieteranfragen an.

Die installierte Version 0.1.3 bestätigt den Start erst nach dem ersten erfolgreichen WebView-Aufruf. Der Paketbericht weist Start, Import, Wiedergabeaufbereitung, Transkription, Inhaltsanalyse und PDF-Export einzeln in Millisekunden aus. Nach der Veröffentlichung startet ein separater Workflow diese Prüfung mit beiden macOS-Architekturen, Windows, dem Ubuntu-Deb-Paket und dem entpackten AppImage. Windows läuft auf zwei separaten VMs mit Standardkonfiguration und deaktivierter automatischer Proxyerkennung. Der Standardtest meldet jeden externen Kontakt der Anwendung oder ihrer Kindprozesse einschließlich DNS als Fehler. Ein Bericht mit den bereits abgeschlossenen Messungen bleibt auch bei einem Netzwerkbefund erhalten. Windows kann den Namen `wpad` mit der systemweiten Proxyerkennung auflösen. Die Vergleichsmessung verändert ausschließlich die Netzwerkkonfiguration der dafür neu bereitgestellten VM. [Microsoft-Dokumentation zu WPAD](https://learn.microsoft.com/en-us/troubleshoot/windows-server/networking/disable-http-proxy-auth-features#how-to-disable-wpad)

Der Workflow **Spoken interface acceptance** installiert das veröffentlichte Ubuntu-Deb-Paket und bedient MeetingPal in einer temporären X11-Sitzung mit Orca. Er prüft den AT-SPI-Fokus und die Reihenfolge der von Orca gesprochenen Beschriftungen für Startschaltfläche, Bibliothekssuche per Strg+K, Aufnahmedialog und Fokusrückgabe per Escape. Jede Aktion läuft mit deutscher und englischer Oberfläche. [Erster Durchlauf mit dem öffentlichen Paket 0.1.2](https://github.com/MaKammi/MeetingPal/actions/runs/45416296119)

## Lokal nachgewiesen (18. November 2026)

- Windows: Frontend-Build, Unit-Tests und Browser-Interaktion erfolgreich.
- Windows: kompletter synthetischer Videoimport → Wiedergabe → Frame-Auswahl → PDF/Markdown/JSON/SRT/VTT erfolgreich.
- Windows: derselbe Encoder wie bei Bildschirmaufnahmen verarbeitet künstliche RGBA-Bilder inklusive Zuschnitt zu drei abspielbaren VP8-Segmenten. Ausgabegröße und undurchsichtiges YUV420P werden geprüft; damit ist der zuvor beobachtete Transparenz-/`auto_alt_ref`-Fehler abgedeckt.
- Windows: echte Whisper-Small-Transkription des öffentlichen Benchmark-Audios, abschließende Transkript-Nachbearbeitung, idempotente Wiederholung und Erhalt manueller Korrekturen erfolgreich. Lokale Qwen3-Auswertung mit schemaerzwungenen Quellen und Meeting-Frage erfolgreich; null Cloud-Anfragen.
- Windows: Desktop-App startet; alle vier gebündelten Laufzeiten werden als bereit erkannt.
- Öffentliches Release 0.1.1: Der Windows-Updater meldete die veröffentlichte Version als aktuell und lud für eine simulierte ältere Installation den vollständigen NSIS-Installer ohne GitHub-Anmeldung. Die Prüfung von Signatur und gebundener Versionsnummer bestand. Der Test führte den Installer nicht aus. Die Anwendung aus dem veröffentlichten Installer enthält das Common-Controls-v6-Manifest, das auch der Windows-Prüfprozess benötigt.
- Windows: Der lokale KI-Integrationstest besteht auch mit absichtlich unerreichbaren `HTTP_PROXY`-/`HTTPS_PROXY`-/`ALL_PROXY`-Zielen und leerem `NO_PROXY`, sobald die Modelle installiert sind. Loopback-Inferenz verwendet einen separaten Client ohne Proxy. Dies ergänzt die noch offene vollständige Netzwerk-/Firewall-Abnahme.
- PDF-Testexport mit Poppler gerendert und visuell geprüft; Umlaute auch nach Textextraktion korrekt, Überschriften, Quellen und Seitennummern lesbar. Die reproduzierbare Vorlage entsteht mit `cargo run --manifest-path src-tauri/Cargo.toml --example export_fixture` unter `test-results/exports`.

Die opt-in Integrationstests stehen unter `src-tauri/tests`. `cargo test --manifest-path src-tauri/Cargo.toml --test pipeline -- --ignored` benötigt die gebündelten Codecs. `--test local_ai -- --ignored` lädt ausdrücklich etwa 3 GB öffentliche Modelle nach `runtime-data/test-models` und prüft die echte lokale KI. Diese Tests zeichnen weder Mikrofon noch Bildschirm auf.
