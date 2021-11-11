use crate::api::model::chat;
use crate::custom_error::{DataType, ErrorCode, Fallible};
use crate::db::get_pool;
use chrono::{DateTime, Utc};

pub async fn get_init_info(id: i64) -> Fallible<chat::InitInfo> {
    let pool = get_pool();
    struct TmpChannel {
        channel_id: i64,
        name_1: String,
        name_2: String,
        user_id_1: i64,
        sender_id: i64,
        time: DateTime<Utc>,
        text: String,
    }
    // TODO: 多重 JOIN 可能導致效能問題
    let channels = sqlx::query_as!(
        TmpChannel,
        "
		SELECT chat.direct_chats.id as channel_id, u1.user_name as name_1, u2.user_name as name_2, user_id_1, m1.create_time as time, m1.content as text, m1.sender_id as sender_id
        FROM chat.direct_chats
        JOIN users u1
        ON chat.direct_chats.user_id_1 = u1.id
        JOIN users u2
        ON chat.direct_chats.user_id_2 = u2.id
        JOIN chat.direct_messages m1
        ON chat.direct_chats.id = m1.direct_chat_id
        LEFT OUTER JOIN chat.direct_messages m2
        ON (chat.direct_chats.id = m2.direct_chat_id
            AND (m1.id < m2.id))
        WHERE (user_id_1 = $1 OR user_id_2 = $1) AND m2.id IS NULL
		",
        id
    ).fetch_all(pool).await?;
    let channels: Vec<chat::Channel> = channels
        .into_iter()
        .map(|tmp| {
            let name = if tmp.user_id_1 == id {
                tmp.name_2.clone()
            } else {
                tmp.name_1.clone()
            };
            let sender_name = if tmp.user_id_1 == tmp.sender_id {
                tmp.name_1.clone()
            } else {
                tmp.name_2.clone()
            };
            chat::Channel::Direct(chat::Direct {
                channel_id: tmp.channel_id,
                name,
                last_msg: chat::Message {
                    sender_name,
                    text: tmp.text,
                    time: tmp.time,
                },
            })
        })
        .collect();
    Ok(chat::InitInfo { channels })
}
pub async fn get_receiver(msg: &chat::MessageSending, id: i64) -> Fallible<i64> {
    let pool = get_pool();
    struct Users {
        user_id_1: i64,
        user_id_2: i64,
    }
    let users = sqlx::query_as!(
        Users,
        "
		SELECT user_id_1, user_id_2 FROM chat.direct_chats
        WHERE id = $1
		",
        msg.channel_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(
        ErrorCode::NotFound(DataType::DirectChannel, "".to_owned()).context("找不到此私訊頻道"),
    )?;
    if users.user_id_1 == id {
        Ok(users.user_id_2)
    } else if users.user_id_2 == id {
        Ok(users.user_id_1)
    } else {
        Err(ErrorCode::PermissionDenied.context("您不屬於此私訊頻道"))
    }
}

// 回傳接收者的用戶 id
pub async fn send_message(msg: &chat::MessageSending, id: i64) -> Fallible<i64> {
    let receiver = get_receiver(msg, id).await?;
    let pool = get_pool();
    sqlx::query!(
        "
		INSERT INTO chat.direct_messages (direct_chat_id, sender_id, content)
		VALUES ($1, $2, $3)
		RETURNING id
		",
        msg.channel_id,
        id,
        msg.content,
    )
    .fetch_one(pool)
    .await?;
    Ok(receiver)
}
pub async fn creat_direct_chat(user_id_1: i64, user_id_2: i64) -> Fallible<i64> {
    let pool = get_pool();
    let chat_id = sqlx::query!(
        "
		INSERT INTO chat.direct_chats (user_id_1, user_id_2)
		VALUES ($1, $2)
		RETURNING id
		",
        user_id_1,
        user_id_2,
    )
    .fetch_one(pool)
    .await?
    .id;
    Ok(chat_id)
}
