export interface AudioDevice {
  id: string;
  name: string;
  process_name: string;
  sample_rate: number;
  channels: number;
  is_active: boolean;
}

export interface PipelineStatus {
  state: "idle" | "running" | "stopped";
  duration_secs: number;
  sentences_transcribed: number;
  sentences_translated: number;
  current_level: number;
}

export interface PipelineEvent {
  type: "Interim" | "Transcribed" | "Translated" | "AudioLevel" | "Error" | "StateChange";
  id?: number;
  text?: string;
  source_text?: string;
  translated_text?: string;
  level?: number;
  message?: string;
  state?: string;
}

export interface TranscriptEntry {
  id: number;           // matches sentence id from backend
  source_text: string;
  translated_text: string;
  timestamp: number;
}

export interface AppSettings {
  asr_engine: string;
  translation_engine: string;
  source_lang: string;
  target_lang: string;
  whisper_model: string;
  deepgram_api_key: string;
  overlay_show_source: boolean;
  overlay_opacity: number;
}

export interface Language {
  code: string;
  name: string;
  flag: string;
}

export const LANGUAGES: Language[] = [
  { code: "en", name: "English", flag: "EN" },
  { code: "ta", name: "Tamil", flag: "TA" },
  { code: "hi", name: "Hindi", flag: "HI" },
  { code: "te", name: "Telugu", flag: "TE" },
  { code: "bn", name: "Bengali", flag: "BN" },
  { code: "es", name: "Spanish", flag: "ES" },
  { code: "fr", name: "French", flag: "FR" },
  { code: "de", name: "German", flag: "DE" },
  { code: "ja", name: "Japanese", flag: "JA" },
  { code: "ko", name: "Korean", flag: "KO" },
  { code: "zh-CN", name: "Chinese", flag: "ZH" },
  { code: "ar", name: "Arabic", flag: "AR" },
  { code: "pt", name: "Portuguese", flag: "PT" },
  { code: "ru", name: "Russian", flag: "RU" },
  { code: "it", name: "Italian", flag: "IT" },
  { code: "vi", name: "Vietnamese", flag: "VI" },
  { code: "th", name: "Thai", flag: "TH" },
  { code: "id", name: "Indonesian", flag: "ID" },
  { code: "nl", name: "Dutch", flag: "NL" },
  { code: "tr", name: "Turkish", flag: "TR" },
];
