//! Domain boundaries for subsequent stages. These are contracts, not working integrations.
use crate::{Result, event::Event, task::Lease};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub role: String,
    pub content: String,
}
pub trait ChatStream {
    fn on_text(&mut self, delta: &str) -> Result<()>;
    fn is_cancelled(&self) -> bool;
}
pub trait ChatProvider {
    fn stream(&mut self, turns: &[ChatTurn], sink: &mut dyn ChatStream) -> Result<()>;
}
/// Implement in native code; no get-key IPC and no key synchronization.
pub trait CredentialStore {
    fn set_secret(&mut self, provider: &str, secret: &[u8]) -> Result<()>;
    fn delete_secret(&mut self, provider: &str) -> Result<()>;
    fn has_secret(&self, provider: &str) -> Result<bool>;
}
pub trait PairedTransport {
    fn send_verified_events(&mut self, events: &[Event]) -> Result<()>;
    fn revoke_peer(&mut self, device_id: &str) -> Result<()>;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerCapabilities {
    pub device_id: String,
    pub capabilities: Vec<String>,
    pub max_concurrency: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepOutcome {
    Checkpoint { step: String, value: Value },
    Paused,
    Finished,
}
pub trait TaskRunner {
    fn capabilities(&self) -> RunnerCapabilities;
    fn execute_step(&mut self, lease: &Lease, checkpoint: Option<&Value>) -> Result<StepOutcome>;
    fn cancel(&mut self, task_id: &str) -> Result<()>;
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySnapshot {
    pub activity_id: String,
    pub source_id: String,
    pub executor_device_id: String,
    pub status: String,
    pub last_updated_ms: i64,
    pub ttl_ms: u64,
    pub playback_position_ms: Option<u64>,
    pub analyzed_ms: Option<u64>,
    pub error_code: Option<String>,
}
pub trait PetRenderer {
    fn render(&mut self, activity: &ActivitySnapshot, expression: &str) -> Result<()>;
}
