import type { Translate } from "./types";

// Stored diagnostics retain their original text so old meetings follow the current interface language.
const german: Record<string, string> = {
  "Allow screen recording for MeetingPal in System Settings, then refresh the sources.":
    "Erlaube MeetingPal die Bildschirmaufnahme in den Systemeinstellungen und aktualisiere danach die Quellen.",
  "Bundled FFmpeg runtime is missing.":
    "Die mitgelieferte FFmpeg-Laufzeit fehlt.",
  "Capture lanes have different segment lengths":
    "Die Aufnahmespuren haben unterschiedliche Abschnittslängen.",
  "Could not start the bundled video encoder":
    "Der mitgelieferte Videoencoder konnte nicht starten.",
  "Desktop audio could not be opened":
    "Der Systemton konnte nicht geöffnet werden.",
  "Display unavailable": "Der Bildschirm ist nicht verfügbar.",
  "Empty capture frame": "Das Aufnahmebild ist leer.",
  "Less than 100 MB free space. Stopping safely.":
    "Es sind weniger als 100 MB frei. Die Aufnahme wird gesichert und beendet.",
  "Microphone could not be opened":
    "Das Mikrofon konnte nicht geöffnet werden.",
  "No preview frame received": "Es wurde kein Vorschaubild empfangen.",
  "No screen frames received. Check screen recording permission.":
    "Es wurden keine Bildschirmbilder empfangen. Prüfe die Berechtigung zur Bildschirmaufnahme.",
  "No video frame received": "Es wurde kein Videobild empfangen.",
  "Recording paused after a system suspension or long scheduling gap. Check your devices, then resume.":
    "Die Aufnahme wurde nach dem Ruhezustand oder einer längeren Unterbrechung pausiert. Prüfe die Geräte, bevor du fortfährst.",
  "Screen recording permission was not granted.":
    "Die Berechtigung zur Bildschirmaufnahme fehlt.",
  "ScreenCaptureKit captures system audio. Select the system audio source.":
    "Wähle die Systemtonquelle für die Aufnahme mit ScreenCaptureKit.",
  "Select the source through the system picker":
    "Wähle die Quelle im Auswahldialog des Systems.",
  "Selected screen or window is no longer available.":
    "Der gewählte Bildschirm oder das Fenster ist nicht mehr verfügbar.",
  "Selected window is unavailable": "Das gewählte Fenster ist nicht verfügbar.",
  "The capture region is outside the selected source. Video capture stopped to avoid recording unintended content.":
    "Der gewählte Ausschnitt liegt außerhalb der Quelle. Die Videoaufnahme wurde angehalten.",
  "The selected display is no longer available":
    "Der gewählte Bildschirm ist nicht mehr verfügbar.",
  "Unsupported preview source": "Diese Quelle unterstützt keine Vorschau.",
  "Video encoder failed. See local encoder.log.":
    "Der Videoencoder ist fehlgeschlagen. Einzelheiten stehen in der lokalen Datei encoder.log.",
  "Video encoder stopped; audio recording continues":
    "Der Videoencoder wurde angehalten. Die Audioaufnahme läuft weiter.",
  "Window unavailable": "Das Fenster ist nicht verfügbar.",
  "Capture dimensions exceed the supported limit":
    "Die Aufnahmeauflösung überschreitet die unterstützte Größe.",
  "Invalid PipeWire descriptor":
    "PipeWire hat eine ungültige Verbindung gemeldet.",
  "Invalid PipeWire format": "Das PipeWire-Format ist ungültig.",
  "Invalid portal image": "Das Systemportal hat ein ungültiges Bild geliefert.",
  "No preview frame from the system picker":
    "Die Systemauswahl hat kein Vorschaubild geliefert.",
  "No screen or window selected": "Wähle einen Bildschirm oder ein Fenster.",
  "Select the portal source again": "Wähle die Quelle im Systemdialog erneut.",
  "Truncated portal video buffer":
    "Das Systemportal hat ein unvollständiges Videobild geliefert.",
  "Unsupported portal pixel format":
    "Das Pixelformat des Systemportals wird nicht unterstützt.",
  "Unsupported video row stride":
    "Der Zeilenabstand des Videobilds wird nicht unterstützt.",
  "ScreenCaptureKit audio permission timed out":
    "Die Zeit für die Audiofreigabe durch ScreenCaptureKit ist abgelaufen.",
  "ScreenCaptureKit system audio requires screen-recording permission.":
    "Für Systemton mit ScreenCaptureKit ist die Berechtigung zur Bildschirmaufnahme erforderlich.",
  "Unsupported ScreenCaptureKit audio format":
    "Das Audioformat von ScreenCaptureKit wird nicht unterstützt.",
  "Choose an absolute export path":
    "Wähle einen vollständigen Dateipfad für den Export.",
  "Invalid note metadata": "Die Angaben zur Notiz sind ungültig.",
  "Invalid request budget": "Das Anfragelimit ist ungültig.",
  "Marker is too long": "Der Text der Zeitmarke ist zu lang.",
  "Media file not found": "Die Mediendatei wurde nicht gefunden.",
  "No active recording": "Es läuft keine Aufnahme.",
  "Not a playable media file":
    "Diese Mediendatei kann nicht wiedergegeben werden.",
  "Processing is still running. Wait for it to finish before deleting this meeting.":
    "Warte mit dem Löschen, bis die Verarbeitung dieses Meetings abgeschlossen ist.",
  "Retention must be between 1 and 3650 days":
    "Die Aufbewahrungsfrist muss zwischen 1 und 3650 Tagen liegen.",
  "Stop recording before removing models":
    "Beende die Aufnahme, bevor du Modelle entfernst.",
  "Stop the recording before deleting it.":
    "Beende die Aufnahme, bevor du sie löschst.",
  "The benchmark currently measures speech recognition.":
    "Der Leistungstest misst die Spracherkennung.",
  "The system capture session has ended. Select the source again.":
    "Die Bildschirmfreigabe ist beendet. Wähle die Quelle erneut.",
  "Transcript edit is too long":
    "Der bearbeitete Transkriptabschnitt ist zu lang.",
  "Transcript segment no longer exists":
    "Dieser Transkriptabschnitt ist nicht mehr vorhanden.",
  "Unknown recording action": "Unbekannter Aufnahmebefehl.",
  "Unsupported interface language":
    "Diese Oberflächensprache wird nicht unterstützt.",
  "Wait for queued AI processing to finish before removing models.":
    "Warte mit dem Entfernen von Modellen, bis die KI-Aufträge abgeschlossen sind.",
  "A recording is already running.": "Es läuft bereits eine Aufnahme.",
  "AI result contains a claim without evidence; please retry.":
    "Das KI-Ergebnis enthält eine Aussage ohne Beleg. Starte die Auswertung erneut.",
  "AI result contains an unknown evidence reference; please retry.":
    "Das KI-Ergebnis verweist auf einen unbekannten Beleg. Starte die Auswertung erneut.",
  "Enable video recording for visual analysis.":
    "Aktiviere die Videoaufnahme für die Bildauswertung.",
  "Enter a meeting title (maximum 250 characters).":
    "Gib einen Meeting-Titel mit höchstens 250 Zeichen ein.",
  "Final transcription requires local Whisper or OpenAI.":
    "Wähle lokales Whisper oder OpenAI für die endgültige Transkription.",
  "Image analysis requires the local vision model.":
    "Für die lokale Bildanalyse wird das Bildmodell benötigt.",
  "Invalid region dimensions.": "Die Maße des Aufnahmebereichs sind ungültig.",
  "Native video analysis requires Google Gemini and cloud access.":
    "Für die Auswertung nativer Videos sind Google Gemini und Cloud-Zugriff erforderlich.",
  "Offline mode requires a local final transcription model.":
    "Wähle für das endgültige Offline-Transkript ein lokales Modell.",
  "Offline mode requires local models for every enabled task.":
    "Wähle für jede aktivierte Offline-Aufgabe ein lokales Modell.",
  "Please confirm that recording consent has been obtained.":
    "Bestätige, dass die Einwilligung zur Aufnahme vorliegt.",
  "Request budget must be between 1 and 100000.":
    "Das Anfragelimit muss zwischen 1 und 100.000 liegen.",
  "Select a Whisper model for speech recognition.":
    "Wähle für die Spracherkennung ein Whisper-Modell.",
  "Select a capture region.": "Wähle einen Aufnahmebereich.",
  "Select a final transcription model.":
    "Wähle ein Modell für das endgültige Transkript.",
  "Select a language model for analysis.":
    "Wähle ein Sprachmodell für die Auswertung.",
  "Select a model for each task.": "Wähle für jede Aufgabe ein Modell.",
  "Transcription requires a local Whisper model or OpenAI.":
    "Für die Transkription wird ein lokales Whisper-Modell oder OpenAI benötigt.",
  "Unsupported language": "Diese Sprache wird nicht unterstützt.",
  "Invalid PDF font": "Die PDF-Schrift ist ungültig.",
  "Unsupported export format": "Das Exportformat wird nicht unterstützt.",
  "Gemini could not process the video":
    "Gemini konnte das Video nicht verarbeiten.",
  "Gemini omitted upload session":
    "Gemini hat keine Upload-Sitzung bereitgestellt.",
  "Gemini video part must be between 1 byte and 2 GB":
    "Ein Videoteil für Gemini muss zwischen 1 Byte und 2 GB groß sein.",
  "Invalid Gemini file ID": "Die Gemini-Dateikennung ist ungültig.",
  "Invalid Gemini file name": "Der Gemini-Dateiname ist ungültig.",
  "Invalid Google upload endpoint": "Die Google-Uploadadresse ist ungültig.",
  "Gemini video processing timed out":
    "Die Zeit für die Videoverarbeitung durch Gemini ist abgelaufen.",
  "Audio segment no longer exists":
    "Dieser Audioabschnitt ist nicht mehr vorhanden.",
  "Unknown processing job": "Unbekannter Verarbeitungsauftrag.",
  "Please stop the recording before closing MeetingPal.":
    "Beende die Aufnahme, bevor du MeetingPal schließt.",
  "Expected mono PCM16 WAV.":
    "Es wird eine Mono-WAV-Datei mit 16-Bit-PCM benötigt.",
  "Invalid audio segment format":
    "Das Format des Audioabschnitts ist ungültig.",
  "Invalid media filename": "Der Mediendateiname ist ungültig.",
  "Invalid video segment index":
    "Das Verzeichnis der Videoabschnitte ist ungültig.",
  "Media operation timed out":
    "Die Zeit für die Medienverarbeitung ist abgelaufen.",
  "Missing clip filename": "Der Dateiname des Videos fehlt.",
  "Missing video clip": "Ein Videoclip fehlt.",
  "No media streams found": "Die Datei enthält keine Audio- oder Videospur.",
  "Select an existing audio or video file.":
    "Wähle eine vorhandene Audio- oder Videodatei.",
  "The imported file has no audio track.":
    "Die importierte Datei enthält keine Audiospur.",
  "This meeting has no audio files.":
    "Dieses Meeting enthält keine Audiodateien.",
  "Unsupported media extension.":
    "Dieses Mediendateiformat wird nicht unterstützt.",
  "Cancel the download first.": "Brich zuerst den Download ab.",
  "Download cancelled. Partial files are never used as models.":
    "Der Download wurde abgebrochen. Das Modell kann nach einem vollständigen Download verwendet werden.",
  "Download exceeded the catalog size.":
    "Der Download überschreitet die im Katalog angegebene Dateigröße.",
  "Invalid model manifest": "Die Modellbeschreibung ist ungültig.",
  "Model checksum mismatch. The download will not be loaded.":
    "Die Prüfsumme des Modells stimmt nicht. Lade das Modell erneut herunter.",
  "Not enough disk space for this model.":
    "Für dieses Modell ist zu wenig Speicherplatz frei.",
  "This model is already downloading.":
    "Dieses Modell wird bereits heruntergeladen.",
  "Unknown model": "Unbekanntes Modell.",
  "Audio transcription requires OpenAI":
    "Für die Cloud-Transkription wird OpenAI benötigt.",
  "Choose a cloud model for this test":
    "Wähle ein Cloud-Modell für diesen Test.",
  "Missing summary for the test transcript":
    "Für das Testtranskript fehlt die Zusammenfassung.",
  "No streaming transcript received":
    "Es wurden keine laufenden Transkriptergebnisse empfangen.",
  "Provider file cleanup did not complete":
    "Die Testdatei konnte beim Anbieter nicht vollständig entfernt werden.",
  "The model did not recognize the public test speech":
    "Das Modell hat die öffentliche Sprachprobe nicht erkannt.",
  "The provider returned an empty visual description":
    "Der Anbieter hat eine leere Bildbeschreibung zurückgegeben.",
  "Cloud access is disabled for this meeting.":
    "Der Cloud-Zugriff für dieses Meeting ist deaktiviert.",
  "Cloud access is disabled or consent is missing for this meeting.":
    "Der Cloud-Zugriff ist deaktiviert oder die Einwilligung für dieses Meeting fehlt.",
  "Could not start local language model":
    "Das lokale Sprachmodell konnte nicht starten.",
  "Empty local model response":
    "Das lokale Modell hat keine Antwort geliefert.",
  "Enter a question (maximum 2000 characters).":
    "Gib eine Frage mit höchstens 2000 Zeichen ein.",
  "Expected a JSON object":
    "Das Modell hat ein ungültiges Antwortformat geliefert.",
  "File input requires Gemini": "Die Dateiauswertung erfordert Gemini.",
  "Image exceeds the inline size limit":
    "Das Bild ist für die direkte Übertragung zu groß.",
  "Invalid provider model identifier":
    "Die Modell-ID des Anbieters ist ungültig.",
  "Local model startup timed out.":
    "Der Start des lokalen Modells dauerte zu lange.",
  "No model list returned": "Der Anbieter hat keine Modellliste geliefert.",
  "No transcript or visual evidence is available yet.":
    "Für die Auswertung liegen noch keine Transkriptabschnitte oder Bildbelege vor.",
  "No visual description returned":
    "Es wurde keine Bildbeschreibung zurückgegeben.",
  "Request budget reached. Recording continues; increase the budget to retry analysis.":
    "Das Anfragelimit ist erreicht. Die Aufnahme läuft weiter. Erhöhe das Limit und starte die Auswertung erneut.",
  "Select native Gemini video analysis before uploading":
    "Wähle vor dem Upload die native Videoauswertung mit Gemini.",
  "The answer omitted its source references.":
    "In der Antwort fehlen die Quellenangaben.",
  "The answer refers to unknown evidence.":
    "Die Antwort verweist auf unbekannte Belege.",
  "The local model could not start. Check available memory and local-runtime.log.":
    "Das lokale Modell konnte nicht starten. Prüfe den Arbeitsspeicher und die Datei local-runtime.log.",
  "The model did not return valid structured output. Retry or choose another model.":
    "Das Modell hat kein gültiges strukturiertes Ergebnis geliefert. Versuche es erneut oder wähle ein anderes Modell.",
  "The model returned no summary for the available evidence. Retry or select another model.":
    "Das Modell hat die Belege nicht zusammengefasst. Versuche es erneut oder wähle ein anderes Modell.",
  "The model returned no text.": "Das Modell hat keinen Text geliefert.",
  "The provider returned no usable text. Check the model's capabilities.":
    "Der Anbieter hat keinen verwendbaren Text geliefert. Prüfe, ob das Modell die Aufgabe unterstützt.",
  "Unsupported transcription provider":
    "Dieser Anbieter unterstützt die Transkription nicht.",
  "Whisper returned no transcript": "Whisper hat kein Transkript geliefert.",
  "Invalid API key.": "Der API-Schlüssel ist ungültig.",
  "Local models do not require an API key.":
    "Lokale Modelle benötigen keinen API-Schlüssel.",
  "Invalid meeting identifier": "Die Meeting-ID ist ungültig.",
  "Invalid relative path": "Der relative Dateipfad ist ungültig.",
  "Path leaves the meeting folder":
    "Der Dateipfad führt aus dem Meeting-Ordner heraus.",
  "Recording interrupted. Completed segments were recovered; the final unfinished segment may be missing.":
    "Die Aufnahme wurde unterbrochen. Abgeschlossene Abschnitte wurden wiederhergestellt. Der zuletzt begonnene Abschnitt kann fehlen.",
  "This MeetingPal library is already open in another app instance.":
    "Diese Bibliothek ist in einem anderen MeetingPal-Fenster geöffnet.",
  "Invalid segment timestamp": "Die Zeitmarke des Abschnitts ist ungültig.",
  "Missing final transcript": "Das endgültige Transkript fehlt.",
  "Missing segment text": "Der Text des Abschnitts fehlt.",
  "Missing segment timestamp": "Die Zeitmarke des Abschnitts fehlt.",
  "Missing speaker label": "Die Sprecherkennung fehlt.",
  "Missing speaker segment text": "Der Text des Sprecherabschnitts fehlt.",
  "Missing timed transcript segments":
    "Die Transkriptabschnitte mit Zeitmarken fehlen.",
  "Missing transcript delta": "Ein Teil des laufenden Transkripts fehlt.",
  "Missing transcript segments for nonempty speech":
    "Für den erkannten Text fehlen die zugehörigen Abschnitte.",
  "Missing transcription text": "Der Transkriptionstext fehlt.",
  "incomplete transcription stream":
    "Die laufende Transkription ist unvollständig.",
  "transcription stream interrupted":
    "Die laufende Transkription wurde unterbrochen.",
  "Reversed transcript segment":
    "Die Endzeit des Transkriptabschnitts liegt vor der Startzeit.",
  "Transcription event is too large":
    "Ein Datenblock der Transkription ist zu groß.",
  "Transcription stream failed":
    "Die laufende Transkription ist fehlgeschlagen.",
};

export function diagnostic(message: string, t: Translate): string {
  const raw = message.replace(/^Error: /, "");
  const retry = raw.startsWith("Retryable: ");
  const text = retry ? raw.slice(11) : raw;
  let translated = Object.hasOwn(german, text) ? german[text] : undefined;
  if (!translated) {
    const patterns: [RegExp, string][] = [
      [
        /^Provider returned HTTP (\d+)\. Check the key, model availability and quota\.$/,
        "Der Anbieter meldet HTTP $1. Prüfe API-Schlüssel, Modellzugriff und Kontingent.",
      ],
      [
        /^Model (.+) is not installed\. Download it in Models\.$/,
        "Das Modell $1 ist noch nicht installiert. Lade es unter KI-Modelle herunter.",
      ],
      [
        /^No API key available for (.+)\. Add it in Settings\.$/,
        "Für $1 fehlt der API-Schlüssel. Hinterlege ihn in den Einstellungen.",
      ],
      [
        /^Media process failed \((.+)\)\. Check the source format and available disk space\.$/,
        "Die Medienverarbeitung mit $1 ist fehlgeschlagen. Prüfe Dateiformat und freien Speicher.",
      ],
      [
        /^The model omitted the required (.+) section\. Retry or select another model\.$/,
        "In der Modellantwort fehlt der Abschnitt $1. Versuche es erneut oder wähle ein anderes Modell.",
      ],
      [
        /^The bundled (.+) runtime is missing\..+$/,
        "Die mitgelieferte Laufzeit $1 fehlt. Installiere das vollständige MeetingPal-Paket.",
      ],
      [
        /^Capture process stopped unexpectedly \((.+)\)\. Completed segments are preserved\. Check capture-worker\.log\.$/,
        "Der Aufnahmeprozess wurde unerwartet beendet ($1). Abgeschlossene Abschnitte sind gesichert. Einzelheiten stehen in capture-worker.log.",
      ],
      [
        /^Video encoding stopped: (.+)\. Audio continues\.$/,
        "Die Videocodierung wurde angehalten ($1). Die Audioaufnahme läuft weiter.",
      ],
      [
        /^Video source lost: (.+)\. Audio continues; missing video is black\.$/,
        "Die Videoquelle ist ausgefallen ($1). Die Tonaufnahme läuft weiter. Fehlende Bilder erscheinen schwarz.",
      ],
      [
        /^Gemini file cleanup: (.+)$/,
        "Gemini konnte die hochgeladene Datei nicht löschen ($1).",
      ],
      [/^Recording failed: (.+)$/, "Die Aufnahme ist fehlgeschlagen ($1)."],
      [
        /^(Microphone|Desktop audio): no audio samples received for three seconds\..+$/,
        "$1 liefert seit drei Sekunden keinen Ton. Prüfe das Gerät und die Aufnahmeberechtigung.",
      ],
      [
        /^(Microphone|Desktop audio): (\d+) audio chunks were lost; a gap has been preserved\.$/,
        "Bei $1 fehlen $2 Audioblöcke. Die Lücke ist auf der Zeitachse erhalten.",
      ],
    ];
    const entry = patterns.find(([pattern]) => pattern.test(text));
    if (entry)
      translated = text
        .replace(entry[0], entry[1])
        .replace("Microphone", "Mikrofon")
        .replace("Desktop audio", "Systemton");
  }
  if (!translated) return message;
  return t(
    translated + (retry ? " Der Auftrag kann wiederholt werden." : ""),
    raw,
  );
}
