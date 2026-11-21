//! OpenAI speech request format, used only by a configured speech provider.
use companion_secrets::Secret;
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const OPENAI_SPEECH: &str = "https://api.openai.com/v1/audio/speech";
const WAV_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpeechProvider {
    OpenAi,
    Compatible,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpeechSettings {
    pub provider: SpeechProvider,
    pub endpoint: String,
    pub model: String,
    pub voice_id: String,
    pub speaker_id: String,
}

impl Default for SpeechSettings {
    fn default() -> Self {
        Self {
            provider: SpeechProvider::OpenAi,
            endpoint: OPENAI_SPEECH.into(),
            model: "gpt-4o-mini-tts".into(),
            voice_id: "coral".into(),
            speaker_id: String::new(),
        }
    }
}

fn valid_name(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.trim().is_empty())
        && value.len() <= 160
        && !value.chars().any(char::is_control)
}

impl SpeechSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        let url = Url::parse(&self.endpoint).map_err(|_| "VOICE_CLOUD_SETTINGS")?;
        if self.endpoint.len() > 512
            || url.scheme() != "https"
            || url.domain().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() == "/"
            || (self.provider == SpeechProvider::OpenAi && self.endpoint != OPENAI_SPEECH)
            || !valid_name(&self.model, false)
            || !valid_name(&self.voice_id, false)
            || !valid_name(&self.speaker_id, true)
            || (self.provider == SpeechProvider::OpenAi && !self.speaker_id.is_empty())
        {
            return Err("VOICE_CLOUD_SETTINGS");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpeechBinding {
    pub enabled: bool,
    pub voice_id: String,
    pub auto: bool,
}

impl SpeechBinding {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_name(&self.voice_id, true) {
            return Err("VOICE_CLOUD_SETTINGS");
        }
        Ok(())
    }
}

fn request(
    settings: &SpeechSettings,
    binding: &SpeechBinding,
    text: &str,
    speed: f32,
) -> serde_json::Value {
    let voice = if !binding.voice_id.is_empty() {
        &binding.voice_id
    } else if !settings.speaker_id.is_empty() {
        &settings.speaker_id
    } else {
        &settings.voice_id
    };
    serde_json::json!({
        "model": settings.model, "voice": voice, "input": text,
        "speed": speed.clamp(0.7, 1.3), "response_format": "wav"
    })
}

pub async fn speak(
    settings: &SpeechSettings,
    binding: &SpeechBinding,
    key: &Secret,
    text: &str,
    speed: f32,
) -> Result<Vec<u8>, &'static str> {
    settings.validate()?;
    binding.validate()?;
    if text.trim().is_empty() || text.chars().count() > 280 {
        return Err("VOICE_TEXT_LENGTH");
    }
    let client = Client::builder()
        .redirect(Policy::none())
        .https_only(true)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(35))
        .build()
        .map_err(|_| "VOICE_CLOUD_NETWORK")?;
    let mut response = client
        .post(&settings.endpoint)
        .bearer_auth(key.expose())
        .header("Accept", "audio/wav")
        .json(&request(settings, binding, text, speed))
        .send()
        .await
        .map_err(|_| "VOICE_CLOUD_NETWORK")?;
    match response.status().as_u16() {
        401 | 403 => return Err("VOICE_CLOUD_AUTH"),
        429 => return Err("VOICE_CLOUD_BUSY"),
        200 => {}
        _ => return Err("VOICE_SYNTHESIS_FAILED"),
    }
    if response
        .content_length()
        .is_some_and(|n| n > WAV_LIMIT as u64)
    {
        return Err("VOICE_INVALID_AUDIO");
    }
    let mut audio = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "VOICE_CLOUD_NETWORK")? {
        if audio.len() + chunk.len() > WAV_LIMIT {
            return Err("VOICE_INVALID_AUDIO");
        }
        audio.extend_from_slice(&chunk);
    }
    if audio.len() < 44 || &audio[..4] != b"RIFF" || &audio[8..12] != b"WAVE" {
        return Err("VOICE_INVALID_AUDIO");
    }
    Ok(audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_uses_selected_persona_voice_and_expected_wav_protocol() {
        let settings = SpeechSettings::default();
        assert!(settings.validate().is_ok());
        let binding = SpeechBinding {
            enabled: true,
            voice_id: "authorized-voice".into(),
            auto: false,
        };
        let body = request(&settings, &binding, "测试", 1.0);
        assert_eq!(body["voice"], "authorized-voice");
        assert_eq!(body["model"], "gpt-4o-mini-tts");
        assert_eq!(body["input"], "测试");
        assert_eq!(body["response_format"], "wav");
    }

    #[test]
    fn speech_rejects_credentials_in_urls_and_non_https() {
        for endpoint in [
            "http://example.org/tts",
            "https://user:pass@example.org/tts",
            "https://example.org/tts?token=secret",
            "https://example.org/tts#fragment",
        ] {
            let settings = SpeechSettings {
                provider: SpeechProvider::Compatible,
                endpoint: endpoint.into(),
                ..Default::default()
            };
            assert_eq!(settings.validate(), Err("VOICE_CLOUD_SETTINGS"));
        }
    }
}
