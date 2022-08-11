use futures::{channel::mpsc::Sender, SinkExt, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use tracing::*;

use kube::{
    api::{
        Api, AttachParams, AttachedProcess, DeleteParams, ListParams, PostParams, ResourceExt, TerminalSize,
        WatchEvent,
    },
    Client,
};
use tokio::{io::AsyncWriteExt, select};

// check terminal size every 1 second and send it to channel if different
async fn handle_terminal_size(mut channel: Sender<TerminalSize>) {
    let (mut width, mut height) = crossterm::terminal::size().unwrap();
    channel
        .send(TerminalSize { height, width })
        .await
        .expect("fail to write new size to channel");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let (n_width, n_height) = crossterm::terminal::size().unwrap();
        if n_height != height || n_width != width {
            height = n_height;
            width = n_width;
            channel
                .send(TerminalSize { height, width })
                .await
                .expect("fail to write new size to channel");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    let p: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": {
            "containers": [{
                "name": "example",
                "image": "alpine",
                // Do nothing
                "command": ["tail", "-f", "/dev/null"],
            }],
        }
    }))?;
    // Create pod if don't exist
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    let lp = ListParams::default().fields("metadata.name=example").timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", o.name_any());
            }
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                if s.phase.clone().unwrap_or_default() == "Running" {
                    info!("Ready to attach to {}", o.name_any());
                    break;
                }
            }
            _ => {}
        }
    }

    {
        crossterm::terminal::enable_raw_mode()?;
        let mut attached: AttachedProcess = pods
            .exec(
                "example",
                vec!["sh"],
                &AttachParams::default().stdin(true).tty(true).stderr(false),
            )
            .await?;

        let mut stdin = tokio_util::io::ReaderStream::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();

        let mut output = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
        let mut input = attached.stdin().unwrap();

        let term_tx = attached.terminal_size().unwrap();

        tokio::spawn(handle_terminal_size(term_tx));

        loop {
            select! {
                message = stdin.next() => {
                    match message {
                        Some(Ok(message)) => {
                            input.write(&message).await?;
                        }
                        _ => {
                            break;
                        },
                    }
                },
                message = output.next() => {
                    match message {
                        Some(Ok(message)) => {
                            stdout.write(&message).await?;
                            stdout.flush().await?;
                        },
                        _ => {
                            break
                        },
                    }
                },
            };
        }
        crossterm::terminal::disable_raw_mode()?;
    }

    // Delete it
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    println!("");
    Ok(())
}
