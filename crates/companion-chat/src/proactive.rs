//! One evidence-triggered optional model call. Foreground chat and credential changes cancel it.
use super::*;
impl ChatService {
    pub async fn life_tick(
        self: &Arc<Self>,
        now: i64,
        tick: i64,
        authorized_runner: bool,
    ) -> Result<usize, ChatFailure> {
        let deliveries = {
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
        Ok(deliveries + self.deliver_model_share(now, tick, None).await?)
    }

    /// Production host loop. Advance activity checkpoints promptly and defer only an eligible share.
    pub async fn life_tick_delayed(
        self: &Arc<Self>,
        now: i64,
        tick: i64,
        authorized_runner: bool,
    ) -> Result<usize, ChatFailure> {
        if !authorized_runner {
            return self.life_tick(now, tick, false).await;
        }
        let (candidate, cancel) = {
            let mut active = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            let mut store = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
            store.process_companion_batch()?;
            store.advance_life_deferred(now, tick)?;
            if active.changing_key
                || !active.runs.is_empty()
                || active.background.is_some()
                || active.pending_share.is_some()
            {
                return Ok(0);
            }
            let Some(selection) =
                store.next_proactive_target(now, self.vault.has().unwrap_or(false))?
            else {
                return Ok(0);
            };
            let token = CancellationToken::new();
            active.pending_share = Some(token.clone());
            (selection, token)
        };
        let delay = 5000 + (uuid::Uuid::new_v4().as_u128() % 15001) as u64;
        let started = std::time::Instant::now();
        tokio::select! {
            _ = cancel.cancelled() => {},
            _ = tokio::time::sleep(std::time::Duration::from_millis(delay)) => {},
        }
        let elapsed = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
        let (persona, id, model) = candidate;
        let now = now.saturating_add(elapsed);
        let tick = tick.saturating_add(elapsed);
        {
            let mut active = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            active.pending_share = None;
            if cancel.is_cancelled() || active.changing_key || !active.runs.is_empty() {
                return Ok(0);
            }
            if !model {
                return Ok(self
                    .store
                    .lock()
                    .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
                    .deliver_local_proactive(&persona, &id, now)?
                    as usize);
            }
        }
        self.deliver_model_share(now, tick, Some(&id)).await
    }

    async fn deliver_model_share(
        self: &Arc<Self>,
        now: i64,
        tick: i64,
        selection: Option<&str>,
    ) -> Result<usize, ChatFailure> {
        // Without a usable local key, the local life clock keeps running and no billable job is created.
        let (job, key, cancel, done_tx) = {
            let mut active = self.active.lock().map_err(|_| ChatFailure::new("BUSY"))?;
            if active.changing_key || !active.runs.is_empty() || active.background.is_some() {
                return Ok(0);
            }
            let Ok(key) = self.vault.get() else {
                return Ok(0);
            };
            let job = self
                .store
                .lock()
                .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
                .prepare_proactive_candidate(now, tick, selection)?;
            let Some(job) = job else {
                return Ok(0);
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
        output.map(|sent| sent as usize)
    }
}
