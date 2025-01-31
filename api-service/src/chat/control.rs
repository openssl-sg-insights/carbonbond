use crate::api::model::chat::chat_model_root::{client_trigger, server_trigger};
use crate::chat::message;
use crate::custom_error::Fallible;
use futures::{stream::StreamExt, SinkExt, TryFutureExt};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

static NEXT_CHANNEL_ID: AtomicUsize = AtomicUsize::new(1);

struct Sender(usize, Option<mpsc::UnboundedSender<Message>>);

impl PartialEq for Sender {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Sender {}

use std::hash::{Hash, Hasher};
impl Hash for Sender {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Sender {
    fn mock(id: usize) -> Self {
        Sender(id, None)
    }
}

#[derive(Default, Clone)]
pub struct Users(Arc<RwLock<HashMap<i64, HashSet<Sender>>>>);

impl Users {
    async fn add_tx(&self, id: i64, tx_id: usize, tx: mpsc::UnboundedSender<Message>) {
        let mut users = self.0.write().await;
        let tx_set = users.entry(id).or_insert(HashSet::new());
        tx_set.insert(Sender(tx_id, Some(tx)));
    }
    async fn send_to<S>(&self, id: i64, content: S)
    where
        S: Into<String> + Copy,
    {
        let users = self.0.write().await;
        if let Some(tx_set) = users.get(&id) {
            for tx in tx_set {
                tx.1.as_ref().unwrap().send(Message::text(content)).unwrap();
            }
        }
    }
    pub async fn send_api(&self, id: i64, api: server_trigger::API) {
        self.send_to(id, &serde_json::to_string(&api).unwrap())
            .await
    }
    async fn remove_tx(&self, id: i64, tx_id: usize) {
        let mut users = self.0.write().await;
        if let Entry::Occupied(mut tx_set) = users.entry(id) {
            tx_set.get_mut().remove(&Sender::mock(tx_id));
        }
    }
}

pub async fn user_connected(id: i64, websocket: WebSocket, users: Users) {
    let (mut user_ws_tx, mut user_ws_rx) = websocket.split();
    let (tx, rx) = mpsc::unbounded_channel::<Message>();
    let mut rx = UnboundedReceiverStream::new(rx);
    let tx_id = NEXT_CHANNEL_ID.fetch_add(1, Ordering::Relaxed);

    let init_info = message::get_init_info(id).await;

    let init_info = match init_info {
        Ok(init_info) => server_trigger::API::InitInfo(init_info),
        Err(err) => {
            log::info!("{} 無法取得聊天室初始化訊息：{}", id, err);
            return;
        }
    };

    let agent = tokio::task::spawn(async move {
        user_ws_tx
            .send(Message::text(serde_json::to_string(&init_info).unwrap()))
            .unwrap_or_else(|e| {
                eprintln!("websocket send error: {}", e);
            })
            .await;

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                from_rx = rx.next() => {
                    match from_rx {
                        None => {
                            break;
                        },
                        Some(message) => {
                            user_ws_tx
                                .send(message)
                                .unwrap_or_else(|e| {
                                    eprintln!("websocket 傳輸錯誤: {}", e);
                                })
                                .await;
                        }
                    }
                },
                _ = interval.tick() => {
                    user_ws_tx.send(Message::ping("yo"))
                                .unwrap_or_else(|e| {
                                    eprintln!("websocket ping 錯誤: {}", e);
                                })
                                .await;
                }
            }
        }
    });
    users.add_tx(id, tx_id, tx).await;

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", id, e);
                break;
            }
        };
        match msg.to_str() {
            Ok(msg) => {
                println!("{}: {}", id, msg);
                if let Err(err) = handle_message(msg, id, &users).await {
                    log::warn!("用戶 {} 的 websocket 連線發生錯誤： {}", id, err);
                    break;
                }
            }
            Err(_) => {
                // 忽略非文字訊息
                continue;
            }
        }
    }

    users.remove_tx(id, tx_id).await;
    if let Err(err) = agent.await {
        log::warn!("join agent task 錯誤：{}", err);
    }
}

async fn handle_message(msg: &str, id: i64, users: &Users) -> Fallible<()> {
    let api: client_trigger::API = serde_json::from_str(msg)?;
    match api {
        client_trigger::API::MessageSending(message_sending) => {
            let receiver_id = super::message::send_message(&message_sending, id).await?;
            users.send_to(receiver_id, msg).await;
        }
    }
    Ok(())
}
