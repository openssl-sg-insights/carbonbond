use super::{get_pool, DBObject, ToFallible};
use crate::api::model::forum::{LawyerbcResultMini};
use crate::api::model::forum::{LawyerbcResult};
use crate::api::model::forum::{SignupInvitation, SignupInvitationCredit, User};
use crate::config::get_config;
use crate::custom_error::{DataType, ErrorCode, Fallible};
use crate::email::{self, send_signup_email};
use reqwest;
use serde_json::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use urlencoding::encode;

impl DBObject for User {
    const TYPE: DataType = DataType::User;
}

// XXX: 密切關注 sqlx user defined macro
// XXX: 密切關注 sqlx 什麼時候能把 COUNT(*) 判斷為非空
macro_rules! users {
    ($remain:literal, $($arg:expr),*) => {
        sqlx::query_as!(
            User,
            r#"
            WITH metas AS (SELECT
                users.id,
                users.user_name,
                users.email,
                users.sentence,
                users.energy,
                users.introduction,
                users.gender,
                users.birth_year,
                users.job,
                users.city,
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    to_user = users.id
                    AND kind = 'hate'
                    AND is_public = true) AS "hater_count_public!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    to_user = users.id
                    AND kind = 'hate'
                    AND is_public = false) AS "hater_count_private!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    to_user = users.id
                    AND kind = 'follow'
                    AND is_public = true) AS "follower_count_public!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    to_user = users.id
                    AND kind = 'follow'
                    AND is_public = false) AS "follower_count_private!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    from_user = users.id
                    AND kind = 'hate'
                    AND is_public = true) AS "hating_count_public!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    from_user = users.id
                    AND kind = 'hate'
                    AND is_public = false) AS "hating_count_private!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    from_user = users.id
                    AND kind = 'follow'
                    AND is_public = true) AS "following_count_public!",
                (
                SELECT
                    COUNT(*)
                FROM
                    user_relations
                WHERE
                    from_user = users.id
                    AND kind = 'follow'
                    AND is_public = false) AS "following_count_private!",
                (
                SELECT
                    string_agg(title_authentication_user.title, ',')
                FROM
                    title_authentication_user
                WHERE
                    title_authentication_user.user_id = users.id) AS "titles! : Option<String>"
            FROM users) SELECT * FROM metas "# + $remain,
            $($arg),*
        )
    };
}

pub async fn get_by_id(id: i64) -> Fallible<User> {
    let pool = get_pool();
    let user = users!("WHERE metas.id = $1", id)
        .fetch_one(pool)
        .await
        .to_fallible(id)?;
    Ok(user)
}

pub async fn get_by_name(name: &str) -> Fallible<User> {
    let pool = get_pool();
    let user = users!("WHERE metas.user_name = $1", name)
        .fetch_one(pool)
        .await
        .to_fallible(name)?;
    Ok(user)
}

pub async fn get_signup_token(email: &str) -> Fallible<Option<String>> {
    let pool = get_pool();
    let record = sqlx::query!("SELECT token FROM signup_tokens WHERE email = $1", email)
        .fetch_optional(pool)
        .await?;
    Ok(record.map(|r| r.token))
}
pub async fn get_signup_invitations(user_id: i64) -> Fallible<Vec<SignupInvitation>> {
    let pool = get_pool();
    // SQLX 無法正確推斷 left join 後右表格可能會 NULL ，需手動改名協助其推導
    let invitations = sqlx::query_as!(
        SignupInvitation,
        r#"SELECT signup_tokens.email, signup_tokens.create_time, signup_tokens.is_used, users.user_name as "user_name?"
         FROM signup_tokens
         LEFT JOIN users on signup_tokens.email = users.email
         WHERE inviter_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(invitations)
}
pub async fn get_signup_invitation_credit(user_id: i64) -> Fallible<Vec<SignupInvitationCredit>> {
    let pool = get_pool();
    let credits = sqlx::query_as!(
        SignupInvitationCredit,
        "SELECT id, event_name, credit, create_time
         FROM invitation_credits WHERE user_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(credits)
}
pub async fn query_search_result_from_lawyerbc(
    search_text: String,
) -> Fallible<Vec<LawyerbcResultMini>> {
    let mut map = HashMap::new();
    map.insert("keyword", search_text);

    let client = reqwest::Client::new();
    let response = client
        .post("https://lawyerbc.moj.gov.tw/api/cert/search")
        .json(&map)
        .send()
        .await?;

    if ! response.status().is_success() {
        return Err(ErrorCode::SearchingLawyerbcFail.into());
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct LawyerbcResultMiniResponse {
        pub data: LawyerbcResultMiniResponseData,
    }
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct LawyerbcResultMiniResponseData {
        pub lawyers: Vec<LawyerbcResultMini>,
    }

    let response_text = response.text().await?;
    let lawyerbc_result_mini_response: Result<LawyerbcResultMiniResponse, Error> = serde_json::from_str(response_text.as_str());
    match lawyerbc_result_mini_response {
        Ok(resp) => Ok(resp.data.lawyers),
        Err(_) => Err(ErrorCode::ParsingJson.into()),
    }
}
pub async fn query_detail_result_from_lawyerbc(license_id: String) -> Fallible<LawyerbcResult> {
    let response_text = reqwest::get(format!(
        "{}{}",
        "https://lawyerbc.moj.gov.tw/api/cert/info/",
        encode(&license_id).into_owned()
    ))
    .await?
    .text()
    .await?;

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct LawyerbcResultResponse {
        pub data: Vec<LawyerbcResult>,
    }

    let lawyerbc_result_response: Result<LawyerbcResultResponse, Error> = serde_json::from_str(response_text.as_str());
    match lawyerbc_result_response {
        Ok(resp) => Ok(resp.data[0].clone()),
        Err(_) => Err(ErrorCode::ParsingJson.into()),
    }
}
pub async fn create_signup_token(
    email: &str,
    birth_year: i32,
    gender: &str,
    license_id: &str,
    inviter_id: Option<i64>,
) -> Fallible<()> {
    let mut conn = get_pool().begin().await?;

    log::trace!("1. 若是邀請註冊，檢查發出邀請者的邀請額度是否足夠");
    // 1. 若是邀請註冊，檢查發出邀請者的邀請額度是否足夠
    if let Some(inviter_id) = inviter_id {
        struct Ret {
            remaining: Option<i64>,
        }
        let ret = sqlx::query_as!(
            Ret,
            "SELECT
                (SELECT SUM(credit) FROM invitation_credits WHERE user_id = $1)
                -
                (SELECT COUNT(*) FROM signup_tokens WHERE inviter_id = $1)
                as remaining
            ",
            inviter_id
        )
        .fetch_one(&mut conn)
        .await?;
        match ret.remaining {
            Some(remaining) => {
                if remaining <= 0 {
                    return Err(ErrorCode::CreditExhausted.into());
                }
            }
            None => {
                return Err(ErrorCode::CreditExhausted.into());
            }
        }
    }

    log::trace!("2. 檢查 email 是否被用過");
    // 2. 檢查 email 是否被用過
    let arr = sqlx::query!("SELECT 1 as t from users where email = $1 LIMIT 1", email)
        .fetch_all(&mut conn)
        .await?;
    if arr.len() > 0 {
        return Err(ErrorCode::DuplicateRegister.into());
    }
    let arr = sqlx::query!(
        "SELECT 1 as t from signup_tokens where email = $1 LIMIT 1",
        email
    )
    .fetch_all(&mut conn)
    .await?;
    if arr.len() > 0 {
        return Err(ErrorCode::DuplicateInvitation.into());
    }

    log::trace!("3. 生成 token");
    // 3. 生成 token
    let token = crate::util::generate_token();
    sqlx::query!(
        "INSERT INTO signup_tokens (email, birth_year, gender, license_id, token, inviter_id) VALUES ($1, $2, $3, $4, $5, $6)",
        email,
        birth_year,
        gender,
        license_id,
        token,
        inviter_id
    )
    .execute(&mut conn)
    .await?;

    // 4. 寄信
    send_signup_email(&token, &email).await?;

    conn.commit().await?;
    Ok(())
}

pub async fn send_reset_password_email(email: String) -> Fallible<()> {
    let mut conn = get_pool().begin().await?;
    // 1. 檢查是否有此信箱
    let arr = sqlx::query!("SELECT 1 as t from users where email = $1 LIMIT 1", email)
        .fetch_all(&mut conn)
        .await?;
    if arr.len() == 0 {
        return Err(ErrorCode::NotFound(DataType::Email, email).into());
    }

    // 2. 生成 token
    let token = crate::util::generate_token();
    sqlx::query!(
        "INSERT INTO reset_password (user_id, token) VALUES
        ((SELECT id FROM users WHERE email = $1), $2)",
        email,
        token,
    )
    .execute(&mut conn)
    .await?;

    // 3. 寄信
    email::send_reset_password_email(&token, &email).await?;

    conn.commit().await?;
    Ok(())
}
pub async fn get_email_by_signup_token(token: &str) -> Fallible<Option<String>> {
    let pool = get_pool();
    let record = sqlx::query!("SELECT email FROM signup_tokens WHERE token = $1", token)
        .fetch_optional(pool)
        .await?;
    Ok(record.map(|r| r.email))
}
pub async fn signup_by_token(name: &str, password: &str, token: &str) -> Fallible<i64> {
    log::trace!("使用者用註冊碼註冊");
    let email = get_email_by_signup_token(token).await?;
    if let Some(email) = email {
        let mut conn = get_pool().begin().await?;
        let id = signup(name, password, &email).await?;
        sqlx::query!(
            "UPDATE signup_tokens SET is_used = TRUE WHERE token = $1",
            token
        )
        .execute(&mut conn)
        .await?;
        sqlx::query!(
            "UPDATE users
            SET gender = (SELECT gender FROM signup_tokens WHERE token = $1),
                birth_year = (SELECT birth_year FROM signup_tokens WHERE token = $1)
            WHERE user_name = $2",
            token,
            name
        )
        .execute(&mut conn)
        .await?;
        sqlx::query!(
            "INSERT INTO title_authentication_user (user_id, title) VALUES ($1, $2)",
            id,
            "律師",
        )
        .execute(&mut conn)
        .await?;
        sqlx::query!(
            "INSERT INTO title_authentication_unique_id (title,  unique_id) VALUES ($1, $2)",
            "律師",
            email,
        )
        .execute(&mut conn)
        .await?;
        conn.commit().await?;
        Ok(id)
    } else {
        Err(ErrorCode::NotFound(DataType::SignupToken, token.to_owned()).into())
    }
}

pub fn check_password_length(password: &str) -> Fallible<()> {
    let config = get_config();
    if password.len() > config.account.max_password_length
        || password.len() < config.account.min_password_length
    {
        return Err(ErrorCode::PasswordLength.into());
    }
    Ok(())
}
pub async fn signup(name: &str, password: &str, email: &str) -> Fallible<i64> {
    check_password_length(password)?;
    let (salt, hash) = crate::util::generate_password_hash(password)?;
    log::trace!("生成使用者 {}:{} 的鹽及雜湊", name, email);

    let pool = get_pool();
    let res = sqlx::query!(
        "INSERT INTO users (user_name, password_hashed, salt, email) VALUES ($1, $2, $3, $4) RETURNING id",
        name,
        hash,
        salt.to_vec(),
        email,
    )
    .fetch_one(pool)
    .await?;
    log::trace!("成功新增使用者 {}:{}", name, email);
    Ok(res.id)
}

pub async fn get_user_name_by_reset_password_token(token: &str) -> Fallible<Option<String>> {
    let pool = get_pool();
    let record = sqlx::query!(
        "SELECT user_name FROM reset_password
         JOIN users on reset_password.user_id = users.id
         WHERE token = $1 AND is_used = false",
        token
    )
    .fetch_optional(pool)
    .await?;
    Ok(record.map(|r| r.user_name))
}

pub async fn reset_password_by_token(password: &str, token: &str) -> Fallible<()> {
    log::trace!("使用者重置密碼");
    check_password_length(password)?;
    let pool = get_pool();
    // 更新密碼
    let (salt, hash) = crate::util::generate_password_hash(password)?;
    sqlx::query!(
        "UPDATE users SET (password_hashed, salt) = ($1, $2)
        FROM reset_password
        WHERE reset_password.token = $3 and reset_password.user_id = users.id",
        hash,
        salt,
        token,
    )
    .execute(pool)
    .await?;
    // 設同用戶的所有 token 無效
    sqlx::query!(
        "UPDATE reset_password SET is_used = TRUE
        FROM users
        WHERE reset_password.user_id = (SELECT user_id from reset_password where token = $1)",
        token
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn login(name: &str, password: &str) -> Fallible<User> {
    let pool = get_pool();
    let record = sqlx::query!(
        "SELECT salt, password_hashed from users WHERE user_name = $1",
        name
    )
    .fetch_optional(pool)
    .await?
    .ok_or(ErrorCode::PermissionDenied.context("查無使用者"))?;
    let equal = argon2::verify_raw(
        password.as_bytes(),
        &record.salt,
        &record.password_hashed,
        &argon2::Config::default(),
    )?;
    if equal {
        get_by_name(name).await
    } else {
        Err(ErrorCode::PermissionDenied.context("密碼錯誤"))
    }
}

pub async fn update_sentence(id: i64, sentence: String) -> Fallible<()> {
    let pool = get_pool();
    sqlx::query!(
        "
        UPDATE users
        SET sentence = $1
        WHERE id = $2
        ",
        sentence,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_info(
    id: i64,
    introduction: String,
    job: String,
    city: String,
) -> Fallible<()> {
    let pool = get_pool();
    if introduction.chars().count() > 1000 {
        return Err(ErrorCode::ArgumentFormatError("自我介紹長度".to_owned()).into());
    }
    sqlx::query!(
        "
        UPDATE users
        SET (introduction, job, city) = ($2, $3, $4)
        WHERE id = $1
        ",
        id,
        introduction,
        job,
        city
    )
    .execute(pool)
    .await?;
    Ok(())
}
