# Daten und Privatsphäre

Meetings, Transkripte und Modelle liegen als normale Dateien im persönlichen App-Datenverzeichnis. MeetingPal verschlüsselt die Bibliothek nicht zusätzlich. Benutzerrechte, Laufwerksverschlüsselung und die eigene Backup-Strategie bestimmen den Schutz dieser Dateien.

Das lokale Profil verhindert Cloud-Verarbeitung im Backend, auch bei einem manipulierten oder veralteten UI-Zustand. Es gibt keinen Cloud-Fallback. Modellinstallation ist eine ausdrücklich ausgelöste Verbindung zu Hugging Face. Die installierten KI-Laufzeiten arbeiten lokal; die llama.cpp-Verbindung bleibt auf Loopback. Keine Telemetrie, keine Kalender- oder Team-Synchronisierung.

Die Windows-Oberfläche verwendet Microsoft Edge WebView2. Auf dem Windows-Testsystem mit automatischer Proxyerkennung hat WebView2 beim Start eine DNS-Anfrage nach dem Namen `wpad` gesendet. Diese Anfrage dient der Proxyerkennung des Systems und enthält keine Meeting-Inhalte. MeetingPal verändert die systemweite Proxykonfiguration nicht.

Die automatische Prüfung auf Programmupdates ist beim lokalen Standardprofil zunächst aus. Die Einstellung „Auch bei lokaler KI“ erlaubt eine Anfrage an den öffentlichen GitHub-Releasefeed. „Jetzt nach Updates suchen“ löst eine einzelne Anfrage aus. GitHub erhält dabei übliche Verbindungsdaten einschließlich der IP-Adresse. Der Abruf verwendet keine Meeting-Inhalte, API-Schlüssel oder Kontokennung.

Sind für alle aktiven Aufgaben lokale Modelle gewählt, bleibt das Meeting im lokalen Profil. Bei Auswahl eines Cloud-Anbieters ruft MeetingPal mit dem hinterlegten API-Schlüssel dessen Modellliste ab. Die Liste wird beim Öffnen und während längerer Nutzung aktualisiert. Beim Wechsel zu Gemini-Videoclips aus einem lokalen Meeting erscheint zuerst die Bestätigung für die Cloud-Verarbeitung. Beispiele werden erst über die Schaltfläche zum Modelltest übertragen.

Bei Cloud-Profilen werden die im Aufnahme-Dialog genannten Daten übertragen:

- Für die Transkription erhält der gewählte Anbieter (OpenAI oder Google Gemini) den abgeleiteten Audiomix des jeweiligen Abschnitts. Für das endgültige Transkript ist eine eigene Anbieterwahl möglich.
- Auswertung und Fragen: relevante Transkriptabschnitte, Notizen beziehungsweise Bildbeschreibungen an den gewählten Textanbieter.
- Bilder: ausgewählte, bereits zugeschnittene Video-Frames an das Vision-Modell.
- Die ausdrücklich ausgewählte native Gemini-Videoanalyse überträgt Clips mit Ton über die Files API. MeetingPal wartet auf die Verarbeitung und fordert die Löschung jeder hochgeladenen Datei nach Erfolg und bei Verarbeitungsfehlern an. Nach einem Prozessabbruch oder einer fehlgeschlagenen Löschung gilt die Aufbewahrungsfrist des Anbieters. Das Anfragelimit berücksichtigt Upload, Statusabfragen, Auswertung und Löschung.
- „Mit Testdaten beim Anbieter prüfen“ verwendet eine temporäre, getrennte Bibliothek. Übertragen werden ausschließlich das öffentliche Benchmark-Audio, ein vorgegebener Beispielsatz oder eine synthetische Folie bzw. ein Clip. Die Prüfung nutzt den Schlüssel des ausgewählten Anbieters und kann API-Kosten verursachen.

Die Aufbewahrung auf Anbieter-Seite richtet sich nach dem jeweiligen Vertrag und API-Konto; die App kann diese nicht durch lokale Löschung rückgängig machen. API-Schlüssel gehören in Windows Credential Manager, macOS Keychain oder Linux Secret Service. Ist der sichere Speicher nicht verfügbar, zeigt die App den Sitzungsbetrieb an. Schlüssel werden nicht ins Bibliotheks-Backup aufgenommen.

Vor jeder Aufnahme/Verarbeitung wird die Bestätigung der erforderlichen Einwilligung verlangt. MeetingPal informiert entfernte Teilnehmer nicht automatisch. Desktop-Ton kann Benachrichtigungen und andere Programme enthalten, selbst wenn als Bildquelle nur ein Fenster gewählt wurde. Aufnahme und Übertragung sind deshalb vor dem Meeting zu klären. Der deutsche [§ 201 StGB](https://www.gesetze-im-internet.de/stgb/__201.html) ist insbesondere bei nichtöffentlichen Gesprächen relevant.

Einzelne Medien oder ganze Meetings können gelöscht werden. Automatische Aufbewahrung ist standardmäßig ausgeschaltet; aktivierte Fristen werden beim App-Start auf abgeschlossene Meetings angewendet. Löschung bedeutet normale Dateilöschung, keine garantierte forensische Vernichtung. Kopierte Exporte und Backups bleiben außerhalb der App-Verwaltung.
