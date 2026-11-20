use crate::{Error, Result, storage::Store};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct VoicePreferences {
    pub voice_id: String,
    pub speed_percent: u16,
    pub volume_percent: u8,
    pub muted: bool,
    pub follow_mood: bool,
}

impl Default for VoicePreferences {
    fn default() -> Self {
        Self {
            voice_id: String::new(),
            speed_percent: 100,
            volume_percent: 75,
            muted: false,
            follow_mood: false,
        }
    }
}

impl VoicePreferences {
    pub fn validate(&self) -> Result<()> {
        if self.voice_id.len() > 512
            || self.voice_id.chars().any(char::is_control)
            || !(70..=130).contains(&self.speed_percent)
            || self.volume_percent > 100
        {
            return Err(Error::Invalid("voice preferences"));
        }
        Ok(())
    }

    pub fn expression(&self, mood: &str, intensity_milli: i64) -> (f32, f32) {
        let strength = if self.follow_mood {
            intensity_milli.clamp(0, 1000) as f32 / 1000.0
        } else {
            0.0
        };
        let inflection = match mood {
            "happy" | "curious" | "excited" => strength,
            "sad" | "tired" | "hurt" => -strength,
            _ => 0.0,
        };
        let speed = (f32::from(self.speed_percent) * 0.01 + 0.08 * inflection).clamp(0.7, 1.3);
        (speed, 1.0 + 0.09 * inflection)
    }
}

impl Store {
    pub fn voice_preferences(&self, persona: &str) -> Result<VoicePreferences> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let key = format!("voice:{persona}");
        let row: Option<String> = self
            .conn
            .query_row(
                "SELECT value_json FROM application_settings
             WHERE profile_id=?1 AND device_id=?2 AND key=?3 AND scope='device'",
                params![self.profile_id, self.device_id, key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(row
            .and_then(|json| serde_json::from_str::<VoicePreferences>(&json).ok())
            .filter(|prefs| prefs.validate().is_ok())
            .unwrap_or_default())
    }

    pub fn save_voice_preferences(&self, persona: &str, prefs: &VoicePreferences) -> Result<()> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        prefs.validate()?;
        let key = format!("voice:{persona}");
        self.conn.execute(
            "INSERT INTO application_settings (profile_id,key,value_json,scope,device_id)
             VALUES (?1,?2,?3,'device',?4)
             ON CONFLICT(profile_id,key,device_id) DO UPDATE SET
                value_json=excluded.value_json, revision=application_settings.revision+1
             WHERE application_settings.scope='device' AND application_settings.value_json<>excluded.value_json",
            params![self.profile_id, key, serde_json::to_string(prefs)?, self.device_id],
        )?;
        Ok(())
    }
}

#[derive(Clone, Copy, Serialize)]
pub struct GuideNote {
    midi: u8,
    start_ms: u32,
    duration_ms: u32,
    vowel: &'static str,
}

// Original melody for checking note timing and the playback path.
const GUIDE: [GuideNote; 7] = [
    GuideNote {
        midi: 64,
        start_ms: 0,
        duration_ms: 390,
        vowel: "u",
    },
    GuideNote {
        midi: 67,
        start_ms: 460,
        duration_ms: 390,
        vowel: "o",
    },
    GuideNote {
        midi: 69,
        start_ms: 920,
        duration_ms: 610,
        vowel: "a",
    },
    GuideNote {
        midi: 72,
        start_ms: 1600,
        duration_ms: 330,
        vowel: "a",
    },
    GuideNote {
        midi: 71,
        start_ms: 1980,
        duration_ms: 350,
        vowel: "o",
    },
    GuideNote {
        midi: 67,
        start_ms: 2400,
        duration_ms: 380,
        vowel: "u",
    },
    GuideNote {
        midi: 64,
        start_ms: 2850,
        duration_ms: 820,
        vowel: "u",
    },
];
const RATE: u32 = 24_000;
const GUIDE_MS: u32 = 3800;

pub fn guide_take() -> (&'static [GuideNote], Vec<u8>) {
    let mut pcm = vec![0_i16; (RATE * GUIDE_MS / 1000) as usize];
    for note in GUIDE {
        let start = (note.start_ms * RATE / 1000) as usize;
        let length = (note.duration_ms * RATE / 1000) as usize;
        let fundamental = 440.0_f32 * 2_f32.powf((f32::from(note.midi) - 69.0) / 12.0);
        let harmonics = match note.vowel {
            "a" => [0.46, 0.33, 0.17, 0.10],
            "o" => [0.64, 0.25, 0.08, 0.04],
            _ => [0.80, 0.12, 0.05, 0.02],
        };
        for (frame, sample) in pcm[start..start + length].iter_mut().enumerate() {
            let time = frame as f32 / RATE as f32;
            let envelope = (frame.min(length - 1 - frame) as f32 / (RATE as f32 * 0.040))
                .min(1.0)
                .powi(2);
            let phase = std::f32::consts::TAU * fundamental * time;
            let tone: f32 = harmonics
                .iter()
                .enumerate()
                .map(|(harmonic, weight)| (phase * (harmonic + 1) as f32).sin() * weight)
                .sum();
            *sample = (tone * envelope * 16_000.0) as i16;
        }
    }
    let length = pcm.len() as u32 * 2;
    let mut wav = Vec::with_capacity(length as usize + 44);
    wav.extend(b"RIFF");
    wav.extend((36 + length).to_le_bytes());
    wav.extend(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(b"\x02\0\x10\0data");
    wav.extend(length.to_le_bytes());
    wav.extend(pcm.into_iter().flat_map(i16::to_le_bytes));
    (&GUIDE, wav)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_prefs_stay_on_device_and_separate_by_persona() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("voice.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let default = store.active_persona_id().unwrap();
        let other = store.create_persona("海蓝", "温和好奇").unwrap().id;
        let events = |store: &Store| {
            store
                .conn
                .query_row("SELECT count(*) FROM sync_events", [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap()
        };
        let before = events(&store);
        let prefs = VoicePreferences {
            voice_id: "zh-CN-offline".into(),
            muted: true,
            ..VoicePreferences::default()
        };
        store.save_voice_preferences(&other, &prefs).unwrap();
        assert_eq!(
            store.voice_preferences(&default).unwrap(),
            VoicePreferences::default()
        );
        assert_eq!(before, events(&store));
        assert!(store.save_voice_preferences("unknown", &prefs).is_err());
        assert!(
            store
                .save_voice_preferences(
                    &other,
                    &VoicePreferences {
                        volume_percent: 101,
                        ..prefs.clone()
                    }
                )
                .is_err()
        );
        drop(store);
        assert_eq!(
            Store::open(&path)
                .unwrap()
                .voice_preferences(&other)
                .unwrap(),
            prefs
        );
    }

    #[test]
    fn expression_is_optional_and_bounded() {
        let mut prefs = VoicePreferences {
            speed_percent: 130,
            ..VoicePreferences::default()
        };
        assert_eq!(prefs.expression("happy", 1000), (1.3, 1.0));
        prefs.follow_mood = true;
        assert_eq!(prefs.expression("happy", 3000), (1.3, 1.09));
    }

    #[test]
    fn guide_has_real_notes_and_silence_between_them() {
        let (score, wav) = guide_take();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + (GUIDE_MS * RATE * 2 / 1000) as usize);
        let pcm: Vec<_> = wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .copied()
            .map(i16::from_le_bytes)
            .collect();
        assert!(pcm[9600..10800].iter().all(|sample| *sample == 0));
        for note in score {
            let start = (note.start_ms + 80) as usize * 24;
            let energy = |midi: u8| {
                let hz = 440.0_f64 * 2_f64.powf((f64::from(midi) - 69.0) / 12.0);
                let phase_step = std::f64::consts::TAU * hz / f64::from(RATE);
                let (mut sine, mut cosine) = (0.0, 0.0);
                for (i, sample) in pcm[start..start + 4800].iter().enumerate() {
                    let (sin, cos) = (i as f64 * phase_step).sin_cos();
                    sine += f64::from(*sample) * sin;
                    cosine += f64::from(*sample) * cos;
                }
                (sine * sine + cosine * cosine).sqrt()
            };
            assert!(energy(note.midi) > 10.0 * energy(note.midi - 1).max(energy(note.midi + 1)));
        }
    }
}
