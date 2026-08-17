use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;

use super::TranslationBackend;

const GOOGLE_TRANSLATE_URL: &str = "https://translation.googleapis.com/language/translate/v2";

#[derive(Deserialize)]
struct GoogleResponse {
    data: GoogleData,
}

#[derive(Deserialize)]
struct GoogleData {
    translations: Vec<GoogleTranslation>,
}

#[derive(Deserialize)]
struct GoogleTranslation {
    #[serde(alias = "translatedText")]
    translated_text: String,
}

/// Paid translation backend using Google Cloud Translation API.
/// Requires a valid API key.
pub struct GoogleTranslateBackend {
    client: Client,
    api_key: String,
}

impl GoogleTranslateBackend {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            api_key: api_key.to_string(),
        }
    }
}

impl TranslationBackend for GoogleTranslateBackend {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let response = self.client
            .post(GOOGLE_TRANSLATE_URL)
            .query(&[("key", &self.api_key)])
            .json(&serde_json::json!({
                "q": text,
                "source": source_lang,
                "target": target_lang,
                "format": "text"
            }))
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Google Translate API error ({}): {}",
                status,
                body
            ));
        }

        let result: GoogleResponse = response.json()?;
        result
            .data
            .translations
            .first()
            .map(|t| t.translated_text.clone())
            .ok_or_else(|| anyhow::anyhow!("No translation returned"))
    }

    fn name(&self) -> &str {
        "Google Cloud Translation (Paid)"
    }
}
