import { writable, derived } from "svelte/store";
import type {
  AudioDevice,
  TranscriptEntry,
  PipelineEvent,
} from "../types";

// Pipeline state
export const pipelineState = writable<"idle" | "running" | "error">("idle");
export const audioLevel = writable<number>(0);
export const interimText = writable<string>("");
export const transcript = writable<TranscriptEntry[]>([]);
export const errorMessage = writable<string>("");
export const durationSecs = writable<number>(0);

// Audio devices
export const audioDevices = writable<AudioDevice[]>([]);
export const selectedDevice = writable<AudioDevice | null>(null);

let nextId = 1;

export function clearTranscript() {
  transcript.set([]);
  nextId = 1;
}

export function handlePipelineEvent(event: PipelineEvent) {
  switch (event.type) {

    case "Interim":
      interimText.set(event.text || "");
      break;

    // ASR finished — show transcription immediately, translation pending
    case "Transcribed":
      interimText.set("");
      transcript.update((entries) => [
        ...entries,
        {
          id: event.id ?? nextId++,
          source_text: event.text || "",
          translated_text: "⏳ Translating...",
          timestamp: Date.now(),
        },
      ]);
      break;

    // Translation arrived — update the matching entry by id
    case "Translated":
      transcript.update((entries) =>
        entries.map((e) =>
          e.id === event.id
            ? { ...e, source_text: event.source_text || e.source_text, translated_text: event.translated_text || "" }
            : e
        )
      );
      break;

    case "AudioLevel":
      audioLevel.set(event.level || 0);
      break;

    case "Error":
      errorMessage.set(event.message || "Unknown error");
      setTimeout(() => errorMessage.set(""), 6000);
      break;

    case "StateChange":
      if (event.state === "stopped") {
        pipelineState.set("idle");
      }
      break;
  }
}

export const formattedDuration = derived(durationSecs, ($secs) => {
  const mins = Math.floor($secs / 60);
  const secs = $secs % 60;
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
});
