# Drittanbieter, Quellen und Weitergabe

Die Quellstände der KI-Laufzeiten und die Hashes der FFmpeg-Archive stehen in `resources/runtimes.json`. `scripts/build-runtimes.mjs` kopiert Lizenzen, Provenienz und Quellarchive in das Runtime-Paket. Die App ruft FFmpeg als separaten Prozess auf und verlinkt keine FFmpeg-Bibliotheken in Rust.

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

Die Windows-/Linux-Entwicklungspakete verwenden vorerst überprüfte LGPL-Archive von FFmpeg-Builds. Deren **vollständige korrespondierende Quellen einschließlich aller eingebauten Drittbibliotheken und Build-Skripte** müssen für eine reguläre Weitergabe passend zum konkreten Binary bereitliegen. Ein Link allein ersetzt diese Distributionsprüfung nicht. macOS wird aus FFmpeg-, Opus- und libvpx-Quellen ohne GPL/Nonfree gebaut und die Quellen werden mitkopiert.

Vor externer Verteilung die [FFmpeg-Weitergabehinweise](https://ffmpeg.org/legal.html) und sämtliche Abhängigkeiten prüfen. Private Testpakete sind noch keine abgeschlossene Lizenz- oder Signierungsfreigabe. Für den eigenen MeetingPal-Code wird durch dieses private Repository keine öffentliche Open-Source-Lizenz erteilt.
