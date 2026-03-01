use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use iced::Subscription;
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::task::sipper;
use ipc_channel::ipc::{IpcReceiver, IpcSender};

/// A text message pushed by the renderer over the IPC event channel.
#[derive(Debug, Clone)]
pub struct RendererMessage(pub String);

thread_local! {
    static EVENT_RX: RefCell<Option<Arc<Mutex<IpcReceiver<String>>>>> = const { RefCell::new(None) };
}

fn make_event_subscription() -> impl iced::futures::Stream<Item = RendererMessage> {
    let event_rx = EVENT_RX.with(|cell| cell.borrow().clone().expect("EVENT_RX not set"));
    sipper(async move |mut output| {
        let (tx, mut rx) = mpsc::channel::<RendererMessage>(64);

        tokio::task::spawn_blocking(move || {
            let mut tx = tx;
            loop {
                match event_rx.lock().unwrap().recv() {
                    Ok(msg) => {
                        if tx.try_send(RendererMessage(msg)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        while let Some(msg) = rx.next().await {
            output.send(msg).await;
        }
    })
}

/// iced `Subscription` that receives from the renderer's IPC event channel.
pub fn sub_listener(event_rx: Arc<Mutex<IpcReceiver<String>>>) -> Subscription<RendererMessage> {
    EVENT_RX.with(|cell| {
        *cell.borrow_mut() = Some(event_rx);
    });
    Subscription::run(make_event_subscription)
}

/// Send a SQL command to the renderer via IPC and return the response.
pub async fn execute_sql(
    sql_tx: IpcSender<String>,
    reply_rx: Arc<Mutex<IpcReceiver<String>>>,
    command: String,
) -> String {
    tokio::task::spawn_blocking(move || {
        if let Err(e) = sql_tx.send(command) {
            return format!("Error: IPC send failed: {e}");
        }
        match reply_rx.lock().unwrap().recv() {
            Ok(reply) => reply,
            Err(e) => format!("Error: IPC recv failed: {e}"),
        }
    })
    .await
    .unwrap_or_else(|e| format!("Error: task join failed: {e}"))
}
