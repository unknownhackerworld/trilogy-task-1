import { writable } from "svelte/store";
import type { AppSettings } from "../types";

const defaultSettings: AppSettings = {
  asr_engine: "deepgram",
  translation_engine: "libre",
  source_lang: "en",
  target_lang: "ta",
  whisper_model: "large-v3-turbo",
  deepgram_api_key: "",
  overlay_show_source: true,
  overlay_opacity: 0.92,
};

export const settings = writable<AppSettings>(defaultSettings);
export const sourceLang = writable<string>("en");
export const targetLang = writable<string>("ta");
