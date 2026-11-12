use companion_core::{event::Entity, storage::Store};
use serde_json::json;
use std::{env, fs, process::ExitCode};
use uuid::Uuid;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("check") if args.len() == 3 => {
            let store = Store::open(&args[2])?;
            println!("{}", serde_json::to_string_pretty(&store.diagnostics()?)?);
        }
        Some("lab-sync") if args.len() == 2 => {
            let root = env::temp_dir().join(format!("companion-lab-{}", Uuid::new_v4()));
            fs::create_dir_all(&root)?;
            let outcome = (|| -> Result<(), Box<dyn std::error::Error>> {
                let profile = Uuid::new_v4().to_string();
                let a_id = Uuid::new_v4().to_string();
                let b_id = Uuid::new_v4().to_string();
                let mut a = Store::open_lab(root.join("a.sqlite3"), &a_id, &profile)?;
                let mut b = Store::open_lab(root.join("b.sqlite3"), &b_id, &profile)?;
                a.register_lab_peer(&b_id)?;
                b.register_lab_peer(&a_id)?;
                a.put(
                    Entity::Message,
                    &Uuid::new_v4().to_string(),
                    json!({"role":"user","content":"实验夹具 A：手机离线消息"}),
                )?;
                b.put(
                    Entity::Message,
                    &Uuid::new_v4().to_string(),
                    json!({"role":"user","content":"实验夹具 B：电脑离线消息"}),
                )?;
                let to_b = a.export_after(&b.checkpoints()?)?;
                b.import_lab_batch(&to_b)?;
                let to_a = b.export_after(&a.checkpoints()?)?;
                a.import_lab_batch(&to_a)?;
                assert_eq!(a.message_contents()?, b.message_contents()?);
                assert_eq!(a.diagnostics()?.message_count, 2);
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({"test":"two real SQLite files; in-process lab transport; synthetic fixtures","message_count_each":2,"converged":true,"a_integrity":a.diagnostics()?.integrity,"b_integrity":b.diagnostics()?.integrity,"network_tested":false})
                    )?
                );
                Ok(())
            })();
            fs::remove_dir_all(&root)?;
            outcome?;
        }
        _ => {
            return Err(
                "Usage: companion-node check <local-db-path> | companion-node lab-sync".into(),
            );
        }
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
