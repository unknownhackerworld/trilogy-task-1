use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;

use super::TranslationBackend;

/// Free translation using MyMemory API.
/// No API key required. Limit: ~500 req/day.
/// Docs: https://mymemory.translated.net/doc/spec.php
pub struct LibreTranslateBackend {
    client: Client,
}

#[derive(Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[serde(rename = "responseStatus")]
    response_status: u32,
}

#[derive(Deserialize)]
struct MyMemoryData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

impl LibreTranslateBackend {
    pub fn new(_base_url: Option<&str>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl TranslationBackend for LibreTranslateBackend {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let langpair = format!("{}|{}", source_lang, target_lang);

        let response = self
            .client
            .get("https://api.mymemory.translated.net/get")
            .query(&[("q", text), ("langpair", &langpair)])
            .send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MyMemory API error ({})",
                response.status()
            ));
        }

        let result: MyMemoryResponse = response.json()?;

        if result.response_status != 200 {
            return Err(anyhow::anyhow!(
                "MyMemory returned status {}",
                result.response_status
            ));
        }

        Ok(result.response_data.translated_text)
    }

    fn name(&self) -> &str {
        "MyMemory (Free)"
    }
}
