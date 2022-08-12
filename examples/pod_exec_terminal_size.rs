use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{
        Api, AttachParams, AttachedProcess, DeleteParams, PostParams, ResourceExt, TerminalSize,
    },
    Client, runtime::wait::{await_condition, conditions::is_pod_running},
};
use tokio::{io::AsyncWriteExt, select};

// send the new terminal size to channel when it change
async fn handle_terminal_size(mut channel: Sender<TerminalSize>) {
    let (width, height) = crossterm::terminal::size().unwrap();
    channel
        .send(TerminalSize { height, width })
        .await
        .expect("fail to write new size to channel");

    let mut stream = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::window_change()
    ).expect("fail to create signal handler for SIGWINCH");

    loop {
        // wait for a change
        stream.recv().await;
        let (width, height) = crossterm::terminal::size().unwrap();
        channel
            .send(TerminalSize { height, width })
            .await
            .expect("fail to write new size to channel");

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
    let running = await_condition(pods.clone(), "example", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), running).await?;

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
