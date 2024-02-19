use clap::Parser;
use indexmap::IndexMap;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uaf::{Data, LogMessage, Message, Props};

#[derive(Debug, Parser)]
struct Args {
    #[clap(default_value = "10")]
    max_count: usize,
}

fn log(level: &str, message: &str) {
    let log_message = LogMessage {
        level: level.to_string(),
        message: message.to_string(),
        tags: IndexMap::new(),
    };

    let serialized = serde_json::to_string(&log_message).unwrap();
    eprintln!("{}", serialized);
}

fn send_message<T: Serialize>(message: &T) {
    let serialized = serde_json::to_string(&message).unwrap();
    println!("{}", serialized);
}

#[tokio::main]
async fn main() {
    let Args { max_count, .. } = Args::parse();
    let stdin = tokio::io::stdin();
    let stdin_reader = BufReader::new(stdin);
    let mut lines = stdin_reader.lines();
    let mut count = 0;

    while let Some(line) = lines.next_line().await.unwrap() {
        count += 1;
        match serde_json::from_str(&line).expect("Failed to parse message") {
            Message::Spawned { id, props } => match id {
                Ok(id) => log("info", &format!("Spawned actor {id}")),
                Err(e) => log("error", &format!("Failed to spawn actor ({props:?}): {e}")),
            },
            Message::Data(Data { from, to, payload }) => {
                // Handle incoming data message
                log(
                    "info",
                    &format!(
                        "Received message({count}): {:?}",
                        Data { from, to, payload }
                    ),
                );

                if count == 5 {
                    let message = Data {
                        from: None,
                        to,
                        payload: serde_json::json!({"status": "ok"}),
                    };
                    send_message(&Props {
                        executable: std::path::PathBuf::from("/usr/bin/echo"),
                        args: vec![format!("{}", serde_json::to_string(&message).unwrap())],
                    });
                }

                //

                if count >= max_count {
                    return;
                }

                // Example of sending a message back
                let response = Data {
                    from: None,
                    to,
                    payload: serde_json::json!({"status": "ok", "count": count}),
                };
                send_message(&response);
            }
        }
    }
    log("info", "Actor shutting down");
    tokio::io::stdout().flush().await.unwrap();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
}
