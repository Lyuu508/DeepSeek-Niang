//! One evidence-triggered optional model call. Foreground chat and credential changes cancel it.
use super::*;
impl ChatService {
    pub async fn life_tick(
        self: &Arc<Self>,
        now: i64,
        tick: i64,
        authorized_runner: bool,
    ) -> Result<usize, ChatFailure> {
        let mut deliveries = {
            let mut s = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
            s.process_companion_batch()?;
            s.advance_life_at(now, tick, authorized_runner)?
        };
        if !authorized_runner {
            return Ok(deliveries);
        }
        // Without a usable local key, the local life clock keeps running and no billable job is created.
        let (job, key, cancel, done_tx) = {
            let mut active = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            if active.changing_key || !active.runs.is_empty() || active.background.is_some() {
                return Ok(deliveries);
            }
            let Ok(key) = self.vault.get() else {
                return Ok(deliveries);
            };
            let job = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
                .prepare_proactive_share(now, tick)?;
            let Some(job) = job else {
                return Ok(deliveries);
            };
            let cancel = CancellationToken::new();
            let (done_tx, done_rx) = watch::channel(false);
            active.background = Some(Run {
                conversation: job.persona_id.clone(),
                cancel: cancel.clone(),
                done: done_rx,
            });
            (job, key, cancel, done_tx)
        };
        let started = std::time::Instant::now();
        let result = self
            .provider
            .stream(&key, &job.settings, &job.prompt, &cancel, |_| {
                let Ok(s) = self.store.lock() else {
                    return false;
                };
                s.companion_preferences(&job.persona_id).is_ok_and(|p| {
                    p.proactive_enabled && p.proactive_mode != "off" && p.proactive_model_enabled
                })
            })
            .await;
        let elapsed = started.elapsed().as_millis().min(60000) as i64;
        let output = self
            .store
            .lock()
            .map_err(|_| ChatFailure::new("STORAGE_ERROR"))
            .and_then(|mut s| {
                s.finish_proactive_share(
                    &job,
                    now + elapsed,
                    tick + elapsed,
                    &result.content,
                    result.usage.as_ref(),
                    result.error.as_ref().map(|e| e.code.as_str()),
                )
                .map_err(Into::into)
            });
        if let Ok(mut active) = self.active.lock() {
            active.background = None;
        }
        let _ = done_tx.send(true);
        deliveries += output? as usize;
        Ok(deliveries)
    }
}
