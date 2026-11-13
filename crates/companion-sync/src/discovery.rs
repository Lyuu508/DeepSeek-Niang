use crate::{Error, Result, transport::private};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::IpAddr, sync::Mutex, time::Instant};
pub const SERVICE: &str = "_dscompanion._tcp.local.";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub instance: String,
    pub address: String,
}
pub trait Discovery: Send + Sync {
    fn start(&self, port: u16, instance: &str) -> Result<()>;
    fn candidates(&self) -> Vec<Candidate>;
    fn stop(&self);
    fn error(&self) -> Option<String> {
        None
    }
}
pub struct Mdns {
    daemon: Mutex<Option<mdns_sd::ServiceDaemon>>,
    found: std::sync::Arc<Mutex<BTreeMap<String, (Candidate, Instant)>>>,
}
impl Default for Mdns {
    fn default() -> Self {
        Self {
            daemon: Mutex::new(None),
            found: Default::default(),
        }
    }
}
impl Discovery for Mdns {
    fn start(&self, port: u16, instance: &str) -> Result<()> {
        let daemon = mdns_sd::ServiceDaemon::new().map_err(|_| Error::Discovery)?;
        let info = mdns_sd::ServiceInfo::new(
            SERVICE,
            instance,
            &format!("{instance}.local."),
            "",
            port,
            [("v", "2")].as_slice(),
        )
        .map_err(|_| Error::Discovery)?
        .enable_addr_auto();
        daemon.register(info).map_err(|_| Error::Discovery)?;
        let events = daemon.browse(SERVICE).map_err(|_| Error::Discovery)?;
        let found = self.found.clone();
        std::thread::spawn(move || {
            while let Ok(event) = events.recv() {
                match event {
                    mdns_sd::ServiceEvent::ServiceResolved(info) => {
                        let mut found = found.lock().unwrap_or_else(|e| e.into_inner());
                        for ip in info.get_addresses() {
                            let ip: IpAddr = ip.to_ip_addr();
                            if private(ip) {
                                let address = format!("{ip}:{}", info.get_port());
                                if found.len() >= 64 {
                                    continue;
                                }
                                found.insert(
                                    info.get_fullname().into(),
                                    (
                                        Candidate {
                                            instance: info.get_fullname().into(),
                                            address,
                                        },
                                        Instant::now(),
                                    ),
                                );
                            }
                        }
                    }
                    mdns_sd::ServiceEvent::ServiceRemoved(_, name) => {
                        found
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .remove(&name);
                    }
                    _ => {}
                }
            }
        });
        *self.daemon.lock().map_err(|_| Error::Storage)? = Some(daemon);
        Ok(())
    }
    fn candidates(&self) -> Vec<Candidate> {
        self.found
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|(_, at)| at.elapsed().as_secs() < 120)
            .map(|(c, _)| c.clone())
            .collect()
    }
    fn stop(&self) {
        if let Some(d) = self.daemon.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = d.shutdown();
        }
    }
}
