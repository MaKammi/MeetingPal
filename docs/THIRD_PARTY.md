# Drittanbieter, Quellen und Weitergabe

Die Quellstände der KI-Laufzeiten und die Hashes der FFmpeg-Archive stehen in `resources/runtimes.json`. `scripts/build-runtimes.mjs` kopiert Lizenzen, Provenienz und Quellarchive in das Runtime-Paket. Die App ruft FFmpeg als separaten Prozess auf und verlinkt keine FFmpeg-Bibliotheken in Rust.

Vor jedem Desktop-Build erzeugt `scripts/build-notices.mjs` zusätzlich `THIRD_PARTY_NOTICES.txt` aus den tatsächlich aufgelösten Rust-Abhängigkeiten und den npm-Produktionsabhängigkeiten. Vorhandene Lizenz-/Copyright-Dateien werden vollständig übernommen; die Noto-Schriftlizenz und diese Distributionshinweise werden ebenfalls mitgeliefert. Dieser automatisch erzeugte Bestand ersetzt nicht die Prüfung zusätzlich im Installationsformat enthaltener Systembibliotheken, insbesondere bei Linux-AppImages.

| Bestandteil | Lizenz / Quelle |
|---|---|
| Tauri | MIT / Apache-2.0; Cargo-Lockfile |
| React, Vite | MIT; npm-Lockfile |
| whisper.cpp, llama.cpp | MIT; gepinnte Upstream-Commits |
| scap | MIT; gepatchte Quellen und Original-Lizenz in `third-party/` |
| flexaudio | Siehe Lizenz im gepinnten crates.io-Paket |
| xcap | Siehe Lizenz im Cargo-Lockfile-Paket |
| Noto Sans | SIL Open Font License; `resources/NotoSans-LICENSE.txt` |
| Whisper Small / Large-v3-turbo | MIT; jeweilige Modellkarte im Katalog |
| Qwen3 Text / Vision | Apache-2.0; jeweilige Modellkarte im Katalog |
| VP8 / libvpx | BSD; Upstream-Quellen |
| Opus | BSD; Upstream-Quellen |
| FFmpeg | Gewählter LGPL-Build; keine GPL-/Nonfree-Konfiguration |

Der Textmodell-Download verwendet die GGUF-Quantisierung von Unsloth, mit exakter Revision und SHA-256. Das Vision-Modell kommt aus dem offiziellen Qwen-GGUF-Repository. Das Benchmark-Audio stammt aus dem `samples/jfk.wav` des gepinnten whisper.cpp-Quellstands.

Die Paket-Pipeline baut FFmpeg, Opus und libvpx aus den unter `codecs` gepinnten Quellen. Windows wird mit MinGW auf Ubuntu gebaut, Linux und macOS nativ. GPL, Nonfree, Version-3-Code und Netzwerkfunktionen sind deaktiviert; externe automatische Bibliothekserkennung ist aus. Lizenzen, komplette Quellarchive, Build-Skript, Konfiguration und Commit-Provenienz werden mitkopiert. Vor externer Verteilung bleibt die konkrete Paketprüfung erforderlich.

Der Windows-Entwicklungsbefehl verwendet als schnellere Bootstrap-Option noch ein hashgeprüftes LGPL-Archiv von FFmpeg-Builds. Für weiterzugebende Pakete verwendet die CI ausschließlich den eigenen Quellbuild. Dessen `windows-source-codecs`-Artefakt kann auch lokal in `src-tauri/binaries` entpackt werden. Den Bootstrap-Build nicht als abgeschlossene Distributionsfreigabe behandeln.

Vor externer Verteilung die [FFmpeg-Weitergabehinweise](https://ffmpeg.org/legal.html) und sämtliche Abhängigkeiten prüfen. Private Testpakete sind noch keine abgeschlossene Lizenz- oder Signierungsfreigabe. Für den eigenen MeetingPal-Code wird durch dieses private Repository keine öffentliche Open-Source-Lizenz erteilt.
