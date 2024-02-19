use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use clap::Parser;
use faccess::PathExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use uaf::{Data, LogMessage, Message, Props};
use uuid::Uuid;

#[derive(Debug, Parser)]
struct Args {
    executable: PathBuf,
    args: Vec<String>,
}

lazy_static::lazy_static! {
    static ref ACTORS: Arc<Mutex<HashMap<Uuid, UnboundedSender<Message>>>> = Arc::new(Mutex::new(HashMap::new()));
}

async fn spawn_actor(
    spawn_actor: &Props,
    block: bool,
    tx: UnboundedSender<(Uuid, Props)>,
) -> anyhow::Result<Uuid> {
    if !spawn_actor.executable.is_file() {
        bail!("{} is not a file", spawn_actor.executable.display());
    }

    if !spawn_actor.executable.executable() {
        bail!(
            "{} is not an executable file",
            spawn_actor.executable.display()
        );
    }

    let uuid = Uuid::new_v4();
    let mut child = Command::new(&spawn_actor.executable)
        .args(
            &spawn_actor
                .args
                .iter()
                .map(|s| s.replace("{ACTOR_ID}", &uuid.to_string()))
                .collect::<Vec<_>>(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Reading child's stdout
    if let Some(stdout) = child.stdout.take() {
        spawn(async move {
            let stdout_reader = BufReader::new(stdout);
            let mut lines = stdout_reader.lines();
            while let Some(line) = lines.next_line().await.unwrap() {
                if let Ok(mut message) = serde_json::from_str::<Data>(&line) {
                    if message.from.is_none() {
                        message.from = Some(uuid);
                    }
                    if let Some(other) = ACTORS.lock().await.get(&message.to) {
                        if let Err(e) = other.send(Message::Data(message)) {
                            eprintln!("{e}");
                        }
                    }
                } else if let Ok(message) = serde_json::from_str::<Props>(&line) {
                    tx.send((uuid, message)).unwrap();
                } else {
                    eprintln!("Unknown message type: {line}");
                }
            }
        });
    }

    // Optionally, handle child's stderr for log messages
    if let Some(stderr) = child.stderr.take() {
        spawn(async move {
            let stderr_reader = BufReader::new(stderr);
            let mut lines = stderr_reader.lines();
            while let Some(line) = lines.next_line().await.unwrap() {
                if let Ok(message) = serde_json::from_str::<LogMessage>(&line) {
                    eprintln!(
                        "Child {}: {} [{}]",
                        uuid,
                        message.message,
                        message
                            .tags
                            .into_iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                } else {
                    eprintln!("Child {}: {}", uuid, line);
                }
            }
        });
    }

    if let Some(mut stdin) = child.stdin.take() {
        spawn(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
            spawn(async move {
                while let Some(message) = rx.recv().await {
                    let msg = serde_json::to_string(&message).unwrap();
                    stdin
                        .write_all(format!("{msg}\n").as_bytes())
                        .await
                        .expect("Failed to write to child stdin");
                    stdin.flush().await.expect("Failed to flush child stdin");
                }
            });

            let init_message = Data {
                to: uuid,
                from: None,
                payload: serde_json::json!({
                    "id": uuid,
                }),
            };
            tx.send(Message::Data(init_message))
                .expect("Failed to write to child stdin");

            let mut actors = ACTORS.lock().await;
            actors.insert(uuid, tx);
        });
    }

    if block {
        child.wait().await?;
    }

    Ok(uuid)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args { executable, args } = Args::parse();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Uuid, Props)>();

    {
        let tx = tx.clone();
        spawn(async move {
            loop {
                if let Some((uuid, request)) = rx.recv().await {
                    match spawn_actor(&request, false, tx.clone()).await {
                        Ok(new_uid) => {
                            let message = Message::Spawned {
                                id: Ok(new_uid),
                                props: request,
                            };
                            let actors = ACTORS.lock().await;
                            let actor = actors.get(&uuid).unwrap();
                            actor.send(message).unwrap();
                        }
                        Err(e) => {
                            let message = Message::Spawned {
                                id: Err(e.to_string()),
                                props: request,
                            };
                            let actors = ACTORS.lock().await;
                            let actor = actors.get(&uuid).unwrap();
                            actor.send(message).unwrap();
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    _ = spawn_actor(&Props { executable, args }, true, tx.clone()).await?;

    Ok(())
}
