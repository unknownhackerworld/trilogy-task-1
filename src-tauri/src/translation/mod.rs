pub mod libre;
pub mod google;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub source_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
}

/// Trait for all translation backends.
pub trait TranslationBackend: Send + Sync {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
    fn name(&self) -> &str;
}

/// Cached translation engine wrapping any backend.
pub struct TranslationEngine {
    backend: Box<dyn TranslationBackend>,
    cache: Mutex<HashMap<(String, String, String), String>>,
    max_cache_size: usize,
}

impl TranslationEngine {
    pub fn new(backend: Box<dyn TranslationBackend>) -> Self {
        Self {
            backend,
            cache: Mutex::new(HashMap::new()),
            max_cache_size: 500,
        }
    }

    pub fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<TranslationResult> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(TranslationResult {
                source_text: String::new(),
                translated_text: String::new(),
                source_lang: source_lang.to_string(),
                target_lang: target_lang.to_string(),
            });
        }

        let cache_key = (text.to_string(), source_lang.to_string(), target_lang.to_string());

        // Check cache
        {
            let cache = self.cache.lock();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(TranslationResult {
                    source_text: text.to_string(),
                    translated_text: cached.clone(),
                    source_lang: source_lang.to_string(),
                    target_lang: target_lang.to_string(),
                });
            }
        }

        // Translate
        let translated = self.backend.translate(text, source_lang, target_lang)?;

        // Cache result
        {
            let mut cache = self.cache.lock();
            if cache.len() >= self.max_cache_size {
                // Evict ~20% of cache
                let keys_to_remove: Vec<_> = cache.keys().take(self.max_cache_size / 5).cloned().collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
            cache.insert(cache_key, translated.clone());
        }

        Ok(TranslationResult {
            source_text: text.to_string(),
            translated_text: translated,
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
        })
    }

    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }
}

/// Factory: create translation engine by type.
pub fn create_translation_engine(engine_type: &str, api_key: Option<&str>) -> Result<TranslationEngine> {
    let backend: Box<dyn TranslationBackend> = match engine_type {
        "libre" => Box::new(libre::LibreTranslateBackend::new(None)),
        "google" => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Google API key required"))?;
            Box::new(google::GoogleTranslateBackend::new(key))
        }
        _ => Box::new(libre::LibreTranslateBackend::new(None)),
    };

    Ok(TranslationEngine::new(backend))
}
