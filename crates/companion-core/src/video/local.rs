use super::{AnalysisCoverage, Interval, coverage_ms, plan_segments};
use crate::{
    Error, Result,
    chat::now_ms,
    companion::{excerpt, memory},
    event::sha256_hex,
    storage::Store,
    task::{Lease, log, verify},
};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const VIDEO_SEGMENT_MS: u64 = 8_000;
pub const MAX_VIDEO_MS: u64 = 300_000;
pub const MAX_VIDEO_BYTES: u64 = 256 * 1024 * 1024;
const DAY_MS: i64 = 86_400_000;
const PIPELINE: &str = "local-vision-speech-1";

#[derive(Debug, Clone)]
pub struct LocalMedia {
    pub path: String,
    pub title: String,
    pub checksum: String,
    pub container: String,
    pub bytes: u64,
    pub duration_ms: u64,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoModels {
    pub visual: String,
    pub visual_digest: String,
    pub whisper_path: Option<String>,
    pub whisper_digest: Option<String>,
    pub decoder_revision: String,
}

impl VideoModels {
    pub fn revision(&self) -> String {
        sha256_hex(
            format!(
                "{PIPELINE}|{}|{}|{}|{}",
                self.visual,
                self.visual_digest,
                self.whisper_digest.as_deref().unwrap_or("none"),
                self.decoder_revision
            )
            .as_bytes(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechCue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEvent {
    pub at_ms: u64,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SegmentAnalysis {
    pub samples_ms: Vec<u64>,
    pub audio_ms: u64,
    pub speech: Vec<SpeechCue>,
    pub summary: Option<String>,
    pub events: Vec<VideoEvent>,
    pub uncertainty: Option<String>,
    pub error: Option<String>,
}

impl SegmentAnalysis {
    pub fn decoded(&self, span: Interval) -> Vec<Interval> {
        self.samples_ms
            .iter()
            .map(|&at| Interval {
                start_ms: at,
                end_ms: at.saturating_add(1000).min(span.end_ms),
            })
            .collect()
    }

    fn complete(&self, span: Interval, has_audio: bool) -> Result<bool> {
        Ok(self.summary.is_some()
            && self.error.is_none()
            && coverage_ms(span.end_ms, &self.decoded(span))? == span.end_ms - span.start_ms
            && (!has_audio || self.audio_ms == span.end_ms - span.start_ms))
    }

    fn validate(&self, span: Interval) -> Result<()> {
        let time_valid = |at: u64| at >= span.start_ms && at < span.end_ms;
        if self.samples_ms.len() > 17
            || self.samples_ms.iter().any(|&t| !time_valid(t))
            || self.samples_ms.windows(2).any(|pair| pair[0] >= pair[1])
            || self.audio_ms > span.end_ms - span.start_ms
            || self.events.len() > 8
            || self
                .events
                .iter()
                .any(|event| !time_valid(event.at_ms) || event.description.len() > 900)
            || self.speech.len() > 40
            || self.speech.iter().any(|cue| {
                !time_valid(cue.start_ms)
                    || cue.end_ms <= cue.start_ms
                    || cue.end_ms > span.start_ms + self.audio_ms
                    || cue.text.len() > 1800
            })
            || self.summary.as_ref().is_some_and(|s| {
                s.trim().is_empty() || s.len() > 2400 || self.samples_ms.is_empty()
            })
            || self.uncertainty.as_ref().is_some_and(|s| s.len() > 800)
            || self.error.as_ref().is_some_and(|s| s.len() > 80)
            || serde_json::to_vec(self)?.len() > 65_536
        {
            return Err(Error::Invalid("video segment evidence"));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct VideoTask {
    pub id: String,
    pub title: String,
    pub duration_ms: u64,
    pub has_audio: bool,
    pub state: String,
    pub error: Option<String>,
    pub updated_at_ms: i64,
    pub decoded_ms: u64,
    pub visual_ms: u64,
    pub audio_ms: u64,
    pub samples: usize,
    pub segments: Vec<VideoPart>,
    pub summary: Option<String>,
    pub memory_saved: bool,
}

#[derive(Debug, Serialize)]
pub struct VideoPart {
    pub interval: Interval,
    pub state: String,
    pub evidence: Option<SegmentAnalysis>,
}

pub struct VideoWork {
    pub media: LocalMedia,
    pub models: VideoModels,
    pub pending: Vec<(String, Interval)>,
}

pub struct VideoActivity {
    pub label: &'static str,
    pub progress_milli: i64,
    pub updated_at_ms: i64,
}

fn task_owned(store: &Store, task: &str, persona: &str) -> Result<()> {
    if !store.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM task_queue q JOIN video_tasks v ON v.id=q.id
         JOIN video_local_media m ON m.source_id=v.source_id WHERE q.id=?1 AND q.persona_id=?2
         AND q.authority_device_id=?3 AND m.device_id=?3 AND q.kind='local_video')",
        params![task, persona, store.device_id],
        |row| row.get::<_, bool>(0),
    )? {
        return Err(Error::Untrusted);
    }
    Ok(())
}

fn number(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

fn parts(db: &rusqlite::Connection, task: &str) -> Result<Vec<VideoPart>> {
    let mut query = db.prepare("SELECT s.start_ms,s.end_ms,s.status,a.result_json,a.checksum
        FROM video_segments s LEFT JOIN video_analysis_results a ON a.segment_id=s.id AND a.revision=1
        WHERE s.task_id=?1 ORDER BY s.start_ms")?;
    query
        .query_map([task], |r| {
            Ok((
                Interval {
                    start_ms: number(r, 0)?,
                    end_ms: number(r, 1)?,
                },
                r.get(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .map(|row| {
            let (interval, state, body, checksum) = row?;
            let evidence = body
                .map(|json| {
                    if checksum.as_deref() != Some(sha256_hex(json.as_bytes()).as_str()) {
                        return Err(Error::Conflict);
                    }
                    let data: SegmentAnalysis = serde_json::from_str(&json)?;
                    data.validate(interval)?;
                    Ok(data)
                })
                .transpose()?;
            Ok(VideoPart {
                interval,
                state,
                evidence,
            })
        })
        .collect()
}

fn aggregate(parts: &[VideoPart], length: u64) -> Result<(u64, u64, u64, usize)> {
    let mut decoded = Vec::new();
    let mut vision = Vec::new();
    let mut audio = Vec::new();
    let mut frames = 0;
    for part in parts {
        if let Some(data) = &part.evidence {
            let spans = data.decoded(part.interval);
            frames += data.samples_ms.len();
            decoded.extend_from_slice(&spans);
            if data.summary.is_some() {
                vision.extend(spans);
            }
            if data.audio_ms > 0 {
                audio.push(Interval {
                    start_ms: part.interval.start_ms,
                    end_ms: part.interval.start_ms + data.audio_ms,
                });
            }
        }
    }
    Ok((
        coverage_ms(length, &decoded)?,
        coverage_ms(length, &vision)?,
        coverage_ms(length, &audio)?,
        frames,
    ))
}

fn assert_lease(
    tx: &Transaction<'_>,
    store_id: &str,
    boot: &str,
    epoch: i64,
    lease: &Lease,
    tick: i64,
) -> Result<()> {
    verify(tx, lease, store_id, boot, epoch, tick)?;
    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM task_queue q JOIN video_tasks v ON v.id=q.id
        JOIN video_local_media m ON m.source_id=v.source_id
        WHERE q.id=?1 AND q.kind='local_video' AND m.device_id=?2)",
        params![lease.task_id, store_id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error::Untrusted);
    }
    Ok(())
}

impl Store {
    pub fn current_video_activity(&self, persona: &str) -> Result<Option<VideoActivity>> {
        let row = self.conn.query_row("SELECT p.analysis_state,p.updated_at_ms,s.duration_ms,m.has_audio,
            p.visual_coverage_json,p.audio_coverage_json FROM task_queue q JOIN video_tasks v ON v.id=q.id
            JOIN video_sources s ON s.id=v.source_id JOIN video_local_media m ON m.source_id=v.source_id
            JOIN video_progress p ON p.task_id=q.id WHERE q.persona_id=?1 AND q.authority_device_id=?2
                AND q.state='running' AND q.kind='local_video' AND m.device_id=?2 LIMIT 1",
            params![persona,self.device_id],|r|Ok((r.get::<_,String>(0)?,r.get(1)?,number(r,2)?,r.get::<_,bool>(3)?,
                r.get::<_,String>(4)?,r.get::<_,String>(5)?))).optional()?;
        let Some((state, updated_at_ms, duration_ms, has_audio, visual, audio)) = row else {
            return Ok(None);
        };
        let label = match state.as_str() {
            "decoding" => "正在解码本地视频",
            "analyzing" => "正在分析本地视频",
            "organizing" => "正在整理视频时间线",
            _ => "正在准备本地视频分析",
        };
        let coverage = AnalysisCoverage {
            duration_ms,
            visual: serde_json::from_str(&visual)?,
            audio: serde_json::from_str(&audio)?,
            uncertain: Vec::new(),
            synthesis_saved: false,
        };
        let analyzed = if has_audio {
            coverage.analyzed_ms()?
        } else {
            coverage_ms(duration_ms, &coverage.visual)?
        };
        Ok(Some(VideoActivity {
            label,
            progress_milli: (analyzed * 1000 / duration_ms) as i64,
            updated_at_ms,
        }))
    }

    pub fn import_local_video(
        &mut self,
        persona: &str,
        media: &LocalMedia,
        models: &VideoModels,
    ) -> Result<String> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        if self.active_persona_id()? != persona {
            return Err(Error::Untrusted);
        }
        if media.duration_ms == 0
            || media.duration_ms > MAX_VIDEO_MS
            || media.bytes == 0
            || media.bytes > MAX_VIDEO_BYTES
            || media.path.is_empty()
            || media.path.len() > 32_768
            || media.path.contains('\0')
            || media.title.is_empty()
            || media.title.len() > 600
            || media.checksum.len() != 64
            || !media.checksum.bytes().all(|b| b.is_ascii_hexdigit())
            || !matches!(media.container.as_str(), "mov" | "matroska")
            || media.has_audio && (models.whisper_path.is_none() || models.whisper_digest.is_none())
        {
            return Err(Error::Invalid("video import metadata"));
        }
        let revision = models.revision();
        let profile: String = self.conn.query_row(
            "SELECT profile_id FROM personas WHERE id=?1",
            [persona],
            |r| r.get(0),
        )?;
        let tx = self.conn.transaction()?;
        let old: Option<String> = tx
            .query_row(
                "SELECT s.id FROM video_sources s JOIN video_local_media m ON m.source_id=s.id
            WHERE s.persona_id=?1 AND m.device_id=?2 AND s.media_checksum=?3",
                params![persona, self.device_id, media.checksum],
                |r| r.get(0),
            )
            .optional()?;
        let now = now_ms();
        let source = if let Some(id) = old {
            tx.execute(
                "UPDATE video_local_media SET local_path=?2 WHERE source_id=?1",
                params![id, media.path],
            )?;
            id
        } else {
            let (count,duration):(u64,u64)=tx.query_row("SELECT count(*),coalesce(sum(s.duration_ms),0) FROM video_local_media m
                JOIN video_sources s ON s.id=m.source_id WHERE m.device_id=?1 AND m.imported_at_ms>?2",
                params![self.device_id, now-DAY_MS], |r| Ok((number(r,0)?,number(r,1)?)))?;
            if count >= 3 || duration + media.duration_ms > 600_000 {
                return Err(Error::Invalid("video daily limit"));
            }
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO video_sources(id,profile_id,source_kind,title,media_checksum,duration_ms,authorization_json,persona_id)
                VALUES(?1,?2,'local_file',?3,?4,?5,'{\"user_selected\":true,\"local_processing\":true}',?6)",
                params![id,profile,media.title,media.checksum,media.duration_ms as i64,persona])?;
            tx.execute(
                "INSERT INTO video_local_media VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    id,
                    self.device_id,
                    media.path,
                    media.container,
                    media.bytes as i64,
                    media.has_audio,
                    now
                ],
            )?;
            id
        };
        if let Some(id) = tx
            .query_row(
                "SELECT id FROM video_tasks WHERE source_id=?1 AND analysis_version=?2",
                params![source, revision],
                |r| r.get(0),
            )
            .optional()?
        {
            tx.commit()?;
            return Ok(id);
        }
        let id = Uuid::new_v4().to_string();
        let key = format!("local-video:{source}:{revision}");
        tx.execute("INSERT INTO task_queue(id,profile_id,kind,dedupe_key,authority_device_id,persona_id,authority_epoch,max_attempts)
            VALUES(?1,?2,'local_video',?3,?4,?5,?6,10)",
            params![id,profile,key,self.device_id,persona,self.authority_epoch])?;
        tx.execute("INSERT INTO video_tasks(id,source_id,analysis_version,capability_snapshot_json,model_id,media_available)
            VALUES(?1,?2,?3,?4,?5,1)",params![id,source,revision,serde_json::to_string(models)?,models.visual])?;
        tx.execute(
            "INSERT INTO video_progress(task_id,updated_at_ms) VALUES(?1,?2)",
            params![id, now],
        )?;
        for span in plan_segments(media.duration_ms, VIDEO_SEGMENT_MS, 500)? {
            tx.execute("INSERT INTO video_segments(id,task_id,start_ms,end_ms,media_checksum,status,request_key,sampling_json)
                VALUES(?1,?2,?3,?4,?5,'queued',?6,'{}')",
                params![Uuid::new_v4().to_string(),id,span.start_ms as i64,span.end_ms as i64,media.checksum,format!("{id}:{}",span.start_ms)])?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn video_work(&self, persona: &str, id: &str) -> Result<VideoWork> {
        task_owned(self, id, persona)?;
        let (media,raw)=self.conn.query_row("SELECT m.local_path,s.title,s.media_checksum,m.container,m.byte_length,s.duration_ms,m.has_audio,v.capability_snapshot_json
            FROM video_tasks v JOIN video_sources s ON s.id=v.source_id JOIN video_local_media m ON m.source_id=s.id WHERE v.id=?1",
            [id],|r|Ok((LocalMedia{path:r.get(0)?,title:r.get(1)?,checksum:r.get(2)?,container:r.get(3)?,bytes:number(r,4)?,duration_ms:number(r,5)?,has_audio:r.get(6)?},r.get::<_,String>(7)?)))?;
        let mut query=self.conn.prepare("SELECT id,start_ms,end_ms FROM video_segments WHERE task_id=?1 AND status<>'completed' ORDER BY start_ms")?;
        let pending = query
            .query_map([id], |r| {
                Ok((
                    r.get(0)?,
                    Interval {
                        start_ms: number(r, 1)?,
                        end_ms: number(r, 2)?,
                    },
                ))
            })?
            .collect::<std::result::Result<_, _>>()?;
        Ok(VideoWork {
            media,
            models: serde_json::from_str(&raw)?,
            pending,
        })
    }

    pub fn start_video(&mut self, persona: &str, id: &str, tick: i64) -> Result<Lease> {
        task_owned(self, id, persona)?;
        if self.active_persona_id()? != persona {
            return Err(Error::Untrusted);
        }
        if self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM task_queue WHERE kind='local_video'
            AND state='running' AND authority_device_id=?1)",
            [&self.device_id],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::Conflict);
        }
        match self.task_state(id)?.as_str() {
            "paused" => self.resume_task(id)?,
            "uncertain" => self.reconcile_safe_retry(id)?,
            "failed" => self.retry_failed_task(id)?,
            "queued" => (),
            _ => return Err(Error::StaleLease),
        }
        let device = self.device_id.clone();
        let lease = self.claim_task(id, &device, tick, 60_000)?;
        self.video_stage(&lease, tick, "preparing")?;
        Ok(lease)
    }

    pub fn control_video(&mut self, persona: &str, id: &str, cancel: bool) -> Result<()> {
        task_owned(self, id, persona)?;
        if self.active_persona_id()? != persona {
            return Err(Error::Untrusted);
        }
        if cancel {
            self.cancel_task(id)?;
        } else {
            let tx = self.conn.transaction()?;
            if tx.execute("UPDATE task_queue SET state='paused',fence=fence+1,lease_boot_id=NULL,lease_until_tick=NULL,
                attempt=CASE WHEN state='running' THEN max(attempt-1,0) ELSE attempt END
                WHERE id=?1 AND state IN ('running','queued')",[id])? != 1 { return Err(Error::StaleLease); }
            let fence = tx.query_row("SELECT fence FROM task_queue WHERE id=?1", [id], |r| {
                r.get(0)
            })?;
            log(&tx, id, fence, "VIDEO_PAUSED")?;
            tx.commit()?;
        }
        self.conn.execute(
            "UPDATE video_progress SET updated_at_ms=?2 WHERE task_id=?1",
            params![id, now_ms()],
        )?;
        Ok(())
    }

    pub fn video_stage(&mut self, lease: &Lease, tick: i64, phase: &str) -> Result<()> {
        if !matches!(phase, "preparing" | "decoding" | "analyzing" | "organizing") {
            return Err(Error::Invalid("video phase"));
        }
        let tx = self.conn.transaction()?;
        assert_lease(
            &tx,
            &self.device_id,
            &self.boot_id,
            self.authority_epoch,
            lease,
            tick,
        )?;
        tx.execute("UPDATE video_progress SET analysis_state=?2,updated_at_ms=?3,fence=?4 WHERE task_id=?1",
            params![lease.task_id,phase,now_ms(),lease.fence])?;
        tx.commit()?;
        Ok(())
    }

    pub fn reusable_video_segment(
        &self,
        task: &str,
        part: &str,
    ) -> Result<Option<SegmentAnalysis>> {
        let found=self.conn.query_row("SELECT result.result_json,result.checksum FROM video_segments current
            JOIN video_tasks job ON job.id=current.task_id
            JOIN video_segments saved ON saved.media_checksum=current.media_checksum AND saved.start_ms=current.start_ms
              AND saved.end_ms=current.end_ms AND saved.status='completed'
            JOIN video_tasks other ON other.id=saved.task_id AND other.analysis_version=job.analysis_version
            JOIN task_queue q ON q.id=other.id AND q.authority_device_id=?3
            JOIN video_analysis_results result ON result.segment_id=saved.id AND result.revision=1
            WHERE current.id=?1 AND current.task_id=?2 LIMIT 1",
            params![part,task,self.device_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
        found
            .filter(|(json, checksum)| sha256_hex(json.as_bytes()) == *checksum)
            .map(|(json, _)| Ok(serde_json::from_str(&json)?))
            .transpose()
    }

    pub fn save_video_segment(
        &mut self,
        lease: &Lease,
        tick: i64,
        id: &str,
        data: &SegmentAnalysis,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        assert_lease(
            &tx,
            &self.device_id,
            &self.boot_id,
            self.authority_epoch,
            lease,
            tick,
        )?;
        let (span,has_audio,model)=tx.query_row("SELECT s.start_ms,s.end_ms,m.has_audio,v.model_id
            FROM video_segments s JOIN video_tasks v ON v.id=s.task_id JOIN video_local_media m ON m.source_id=v.source_id
            WHERE s.id=?1 AND s.task_id=?2",params![id,lease.task_id],|r|Ok((Interval{start_ms:number(r,0)?,end_ms:number(r,1)?},r.get::<_,bool>(2)?,r.get::<_,String>(3)?)))?;
        data.validate(span)?;
        let state = if data.complete(span, has_audio)? {
            "completed"
        } else {
            "failed"
        };
        let body = serde_json::to_string(data)?;
        let digest = sha256_hex(body.as_bytes());
        tx.execute("INSERT INTO video_analysis_results(id,task_id,segment_id,revision,result_json,model_id,prompt_version,checksum)
            VALUES(?1,?2,?3,1,?4,?5,?6,?7) ON CONFLICT(task_id,segment_id,revision)
            DO UPDATE SET result_json=excluded.result_json,checksum=excluded.checksum",
            params![Uuid::new_v4().to_string(),lease.task_id,id,body,model,PIPELINE,digest])?;
        tx.execute(
            "UPDATE video_segments SET status=?2,sampling_json=?3 WHERE id=?1",
            params![id, state, serde_json::to_string(&data.samples_ms)?],
        )?;
        let checkpoint = serde_json::json!({"result_sha256":digest,"state":state}).to_string();
        tx.execute("INSERT INTO task_checkpoints(task_id,step_key,fence,checkpoint_json,checksum)
            VALUES(?1,?2,?3,?4,?5) ON CONFLICT(task_id,step_key) DO UPDATE SET
            fence=excluded.fence,checkpoint_json=excluded.checkpoint_json,checksum=excluded.checksum",
            params![lease.task_id,id,lease.fence,checkpoint,sha256_hex(checkpoint.as_bytes())])?;
        let interval_parts = parts(&tx, &lease.task_id)?;
        let mut visual = Vec::new();
        let mut audio = Vec::new();
        let mut gaps = Vec::new();
        for part in &interval_parts {
            if part.state != "completed"
                || part
                    .evidence
                    .as_ref()
                    .is_some_and(|v| v.uncertainty.is_some())
            {
                gaps.push(part.interval);
            }
            if let Some(value) = &part.evidence {
                if value.summary.is_some() {
                    visual.extend(value.decoded(part.interval));
                }
                if value.audio_ms > 0 {
                    audio.push(Interval {
                        start_ms: part.interval.start_ms,
                        end_ms: part.interval.start_ms + value.audio_ms,
                    });
                }
            }
        }
        let furthest = interval_parts
            .iter()
            .filter_map(|part| {
                part.evidence
                    .as_ref()
                    .and_then(|v| v.decoded(part.interval).last().map(|s| s.end_ms))
            })
            .max()
            .unwrap_or(0);
        tx.execute("UPDATE video_progress SET decoded_until_ms=?2,visual_coverage_json=?3,audio_coverage_json=?4,
            uncertain_intervals_json=?5,updated_at_ms=?6,fence=?7 WHERE task_id=?1",
            params![lease.task_id,furthest as i64,serde_json::to_string(&visual)?,serde_json::to_string(&audio)?,serde_json::to_string(&gaps)?,now_ms(),lease.fence])?;
        tx.commit()?;
        Ok(())
    }

    pub fn video_parts(&self, persona: &str, id: &str) -> Result<Vec<VideoPart>> {
        task_owned(self, id, persona)?;
        parts(&self.conn, id)
    }

    pub fn finish_video(
        &mut self,
        lease: &Lease,
        tick: i64,
        summary: Option<&str>,
        failure: Option<&str>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        assert_lease(
            &tx,
            &self.device_id,
            &self.boot_id,
            self.authority_epoch,
            lease,
            tick,
        )?;
        let sections = parts(&tx, &lease.task_id)?;
        let complete = summary.is_some() && sections.iter().all(|v| v.state == "completed");
        let state = if complete { "completed" } else { "failed" };
        let phase = if complete {
            "completed"
        } else if summary.is_some() {
            "partial"
        } else {
            "failed"
        };
        if let Some(summary) = summary {
            let body = serde_json::to_string(&excerpt(summary, 2000))?;
            tx.execute("INSERT INTO video_analysis_results(id,task_id,revision,result_json,model_id,prompt_version,checksum)
                SELECT v.id,v.id,1,?2,v.model_id,?3,?4 FROM video_tasks v WHERE v.id=?1
                ON CONFLICT(id) DO UPDATE SET result_json=excluded.result_json,checksum=excluded.checksum",
                params![lease.task_id,body,PIPELINE,sha256_hex(body.as_bytes())])?;
        }
        if summary.is_some() {
            tx.execute(
                "DELETE FROM video_timeline_events WHERE task_id=?1",
                [&lease.task_id],
            )?;
            let mut events = Vec::new();
            for part in sections {
                if let Some(evidence) = part.evidence {
                    for event in evidence.events {
                        if !events.iter().any(|old: &VideoEvent| {
                            old.at_ms.abs_diff(event.at_ms) < 1000
                                && old.description == event.description
                        }) {
                            events.push(event);
                        }
                    }
                }
            }
            for event in events {
                tx.execute("INSERT INTO video_timeline_events VALUES(?1,?2,?3,?3,'model_observation',?4,0,'{\"kind\":\"sampled_video\"}')",
                    params![Uuid::new_v4().to_string(),lease.task_id,event.at_ms as i64,event.description])?;
            }
        }
        tx.execute("UPDATE task_queue SET state=?2,lease_until_tick=NULL,lease_boot_id=NULL,last_error=?3 WHERE id=?1",
            params![lease.task_id,state,failure.or(if phase=="partial"{Some("VIDEO_PARTIAL_COVERAGE")}else{None})])?;
        tx.execute(
            "UPDATE video_progress SET analysis_state=?2,updated_at_ms=?3 WHERE task_id=?1",
            params![lease.task_id, phase, now_ms()],
        )?;
        log(&tx, &lease.task_id, lease.fence, phase)?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_local_videos(&self, persona: &str) -> Result<Vec<VideoTask>> {
        crate::persona::owner_exists(&self.conn, persona, &self.profile_id)?;
        let mut query=self.conn.prepare("SELECT q.id,s.title,s.duration_ms,m.has_audio,
            CASE WHEN q.state IN ('running','failed') THEN p.analysis_state ELSE q.state END,
            q.last_error,p.updated_at_ms,a.result_json,
            EXISTS(SELECT 1 FROM video_memories vm WHERE vm.task_id=q.id)
            FROM video_tasks v JOIN task_queue q ON q.id=v.id JOIN video_progress p ON p.task_id=q.id
            JOIN video_sources s ON s.id=v.source_id JOIN video_local_media m ON m.source_id=s.id
            LEFT JOIN video_analysis_results a ON a.id=q.id
            WHERE q.persona_id=?1 AND m.device_id=?2 AND q.authority_device_id=?2
            ORDER BY m.imported_at_ms DESC,v.id LIMIT 30")?;
        query
            .query_map(params![persona, self.device_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get(1)?,
                    number(r, 2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get(8)?,
                ))
            })?
            .map(|row| {
                let (
                    id,
                    title,
                    duration_ms,
                    has_audio,
                    state,
                    error,
                    updated_at_ms,
                    summary,
                    memory_saved,
                ) = row?;
                let segments = parts(&self.conn, &id)?;
                let (decoded_ms, visual_ms, audio_ms, samples) = aggregate(&segments, duration_ms)?;
                Ok(VideoTask {
                    id,
                    title,
                    duration_ms,
                    has_audio,
                    state,
                    error,
                    updated_at_ms,
                    decoded_ms,
                    visual_ms,
                    audio_ms,
                    samples,
                    segments,
                    summary: summary.map(|s| serde_json::from_str(&s)).transpose()?,
                    memory_saved,
                })
            })
            .collect()
    }

    pub fn remember_video(&mut self, persona: &str, id: &str) -> Result<()> {
        task_owned(self, id, persona)?;
        if self.active_persona_id()? != persona {
            return Err(Error::Untrusted);
        }
        let tx = self.conn.transaction()?;
        if tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM video_memories WHERE task_id=?1)",
            [id],
            |r| r.get::<_, bool>(0),
        )? {
            return Ok(());
        }
        let (source,title,raw,hash):(String,String,String,String)=tx.query_row("SELECT s.id,s.title,a.result_json,a.checksum
            FROM video_tasks v JOIN video_sources s ON s.id=v.source_id JOIN task_queue q ON q.id=v.id
            JOIN video_analysis_results a ON a.id=v.id WHERE v.id=?1 AND q.state='completed'",
            [id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        if sha256_hex(raw.as_bytes()) != hash {
            return Err(Error::Conflict);
        }
        let summary: String = serde_json::from_str(&raw)?;
        let text = format!(
            "本地视频《{}》的抽帧与语音分析摘要：{}",
            excerpt(&title, 120),
            excerpt(&summary, 1500)
        );
        let memory_id = Uuid::new_v4().to_string();
        memory::write(
            &tx,
            &self.device_id,
            persona,
            &memory_id,
            &text,
            "episodic",
            480,
            false,
            false,
            None,
            "manual",
            "",
            now_ms(),
            true,
        )?;
        tx.execute(
            "INSERT INTO video_memories(id,source_id,memory_id,task_id,reaction_json,share_worthy)
            VALUES(?1,?2,?3,?4,'{}',0)",
            params![Uuid::new_v4().to_string(), source, memory_id, id],
        )?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(has_audio: bool) -> (LocalMedia, VideoModels) {
        (
            LocalMedia {
                path: "local-selected.mp4".into(),
                title: "授权测试".into(),
                checksum: "a".repeat(64),
                container: "mov".into(),
                bytes: 10_000,
                duration_ms: 15_000,
                has_audio,
            },
            VideoModels {
                visual: "qwen3-vl:4b".into(),
                visual_digest: "b".repeat(64),
                whisper_path: has_audio.then(|| "model.bin".into()),
                whisper_digest: has_audio.then(|| "c".repeat(64)),
                decoder_revision: "FFmpeg fixture".into(),
            },
        )
    }

    fn observed(span: Interval, audio_ms: u64) -> SegmentAnalysis {
        SegmentAnalysis {
            samples_ms: (span.start_ms..span.end_ms).step_by(1000).collect(),
            audio_ms,
            summary: Some("片段观察".into()),
            events: vec![VideoEvent {
                at_ms: span.start_ms,
                description: "关键观察".into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn pause_fences_late_result_and_resume_reuses_completed_segment() {
        let mut s = Store::open(":memory:").unwrap();
        let persona = s.active_persona_id().unwrap();
        let (media, models) = source(false);
        let task = s.import_local_video(&persona, &media, &models).unwrap();
        assert_eq!(
            s.import_local_video(&persona, &media, &models).unwrap(),
            task
        );
        let plan = s.video_work(&persona, &task).unwrap().pending;
        assert_eq!(plan.len(), 2);
        let old = s.start_video(&persona, &task, 1).unwrap();
        let (first_id, first_span) = &plan[0];
        s.save_video_segment(&old, 2, first_id, &observed(*first_span, 0))
            .unwrap();
        assert_eq!(
            s.current_video_activity(&persona)
                .unwrap()
                .unwrap()
                .progress_milli,
            533
        );
        s.control_video(&persona, &task, false).unwrap();
        assert!(s.current_video_activity(&persona).unwrap().is_none());
        let (last_id, last_span) = &plan[1];
        assert!(matches!(
            s.save_video_segment(&old, 3, last_id, &observed(*last_span, 0)),
            Err(Error::StaleLease)
        ));
        let again = s.start_video(&persona, &task, 4).unwrap();
        assert_eq!(
            s.video_work(&persona, &task).unwrap().pending,
            vec![(last_id.clone(), *last_span)]
        );
        s.save_video_segment(&again, 5, last_id, &observed(*last_span, 0))
            .unwrap();
        s.finish_video(&again, 6, Some("合并观察"), None).unwrap();
        let status = s.list_local_videos(&persona).unwrap().remove(0);
        assert_eq!(
            (status.state.as_str(), status.visual_ms, status.audio_ms),
            ("completed", 15_000, 0)
        );
        assert!(s.managed_memories(&persona).unwrap().is_empty());
        s.remember_video(&persona, &task).unwrap();
        s.remember_video(&persona, &task).unwrap();
        let saved = s.managed_memories(&persona).unwrap();
        assert_eq!(saved.len(), 1);
        assert!(!saved[0].api_allowed && saved[0].text.contains("合并观察"));
        s.lan_capture(|bytes| Ok(sha256_hex(bytes))).unwrap();
        assert!(
            s.lan_export(0)
                .unwrap()
                .iter()
                .any(|event| event.table == "memories" && event.row["id"] == saved[0].id)
        );
    }

    #[test]
    fn decoded_and_visual_success_cannot_hide_missing_audio() {
        let mut s = Store::open(":memory:").unwrap();
        let persona = s.active_persona_id().unwrap();
        let before: i64 = s
            .conn
            .query_row("SELECT count(*) FROM lan_outbox", [], |r| r.get(0))
            .unwrap();
        let (media, models) = source(true);
        let task = s.import_local_video(&persona, &media, &models).unwrap();
        let lease = s.start_video(&persona, &task, 1).unwrap();
        for (id, span) in s.video_work(&persona, &task).unwrap().pending {
            s.save_video_segment(&lease, 2, &id, &observed(span, 1_000))
                .unwrap();
        }
        s.finish_video(&lease, 3, Some("保留已处理的内容"), None)
            .unwrap();
        let status = s.list_local_videos(&persona).unwrap().remove(0);
        assert_eq!(status.state, "partial");
        assert_eq!(status.visual_ms, 15_000);
        assert_eq!(status.audio_ms, 2_000);
        assert!(s.remember_video(&persona, &task).is_err());
        assert!(s.video_work("different-persona", &task).is_err());
        let outbox: i64 = s
            .conn
            .query_row("SELECT count(*) FROM lan_outbox", [], |r| r.get(0))
            .unwrap();
        assert_eq!(outbox, before);
        s.lan_capture(|bytes| Ok(sha256_hex(bytes))).unwrap();
        assert!(
            s.lan_export(0)
                .unwrap()
                .iter()
                .all(|event| !matches!(event.table.as_str(), "task_queue" | "task_checkpoints"))
        );
    }
}
