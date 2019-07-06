use std::io::stdin;

use carbonbond::user::{email, signup};
use carbonbond::forum;
use carbonbond::party;
use carbonbond::db;

use db::models::*;
use db::schema::users::dsl::*;
use diesel::prelude::*;

fn main() -> Result<(), failure::Error> {
    println!("碳鍵 - 資料庫管理介面");
    let db_conn = db::connect_db();
    let mut opt = 1;
    let p1 = "[0] 結束程式";
    let p2 = "[1] 新增使用者";
    let p3 = "[2] 檢視使用者名單";
    let p4 = "[3] 寄出邀請信";
    let p5 = "[4] 新增看板";
    let p6 = "[5] 新增政黨";
    while opt != 0 {
        println!("{}", p1);
        println!("{}", p2);
        println!("{}", p3);
        println!("{}", p4);
        println!("{}", p5);
        println!("{}", p6);
        let mut buff = String::new();
        stdin().read_line(&mut buff)?;
        if let Ok(_opt) = buff.replace("\n", "").parse::<u8>() {
            opt = _opt;
            if opt == 0 {
                break;
            } else if opt == 1 {
                loop {
                    println!("> {}", p2);
                    println!("> 請輸入 信箱 名字 密碼，或輸入空白行回到選單");
                    buff.clear();
                    stdin().read_line(&mut buff)?;
                    let words: Vec<&str> = buff.split_whitespace().collect();
                    if words.len() == 0 {
                        break;
                    }
                    if words.len() != 3 {
                        println!("輸入格式錯誤");
                    } else {
                        signup::create_user(&db_conn, words[0], words[1], words[2]);
                        println!("成功新增使用者：{}", words[0]);
                    }
                }
            } else if opt == 2 {
                let results = users
                    //.filter(id.eq("石墨"))
                    .load::<User>(&db_conn)
                    .expect("取使用者失敗");
                for user in results {
                    println!(
                        "id: {} email: {} 推薦額度: {}",
                        user.id, user.email, user.invitation_credit
                    );
                }
                println!("\n");
            } else if opt == 3 {
                loop {
                    println!("> {}", p4);
                    println!("> 請輸入欲邀請人的信箱，或輸入空白行回到選單");
                    buff.clear();
                    stdin().read_line(&mut buff)?;
                    let words: Vec<&str> = buff.split_whitespace().collect();
                    if words.len() == 0 {
                        break;
                    }
                    if words.len() != 1 {
                        println!("輸入格式錯誤");
                    } else {
                        let invite_code = signup::create_invitation(&db_conn, None, words[0])
                            .expect("無法建立邀請");
                        email::send_invite_email(None, &invite_code, words[0])
                            .expect("寄送邀請信失敗");
                    }
                }
            } else if opt == 4 {
                println!("> {}", p5);
                println!("> 請輸入看板名，或輸入空白行回到選單");
                buff.clear();
                stdin().read_line(&mut buff)?;
                let words: Vec<&str> = buff.split_whitespace().collect();
                if words.len() == 0 {
                    break;
                }
                if words.len() != 1 {
                    println!("輸入格式錯誤");
                } else {
                    forum::create_board(&db_conn, -1, words[0])?;
                    println!("成功新增看板：{}", words[0]);
                }
            } else if opt == 5 {
                println!("> {}", p6);
                println!("> 請輸入 黨名 [所屬看板]，或輸入空白行回到選單");
                buff.clear();
                stdin().read_line(&mut buff)?;
                let words: Vec<&str> = buff.split_whitespace().collect();
                match words.len() {
                    0 => break,
                    1 => {
                        party::create_party(&db_conn, None, words[0])?;
                        println!("成功新增政黨：{}", words[0]);
                    }
                    2 => {
                        party::create_party_with_board_name(&db_conn, Some(words[1]), words[0])?;
                        println!("成功新增 {}版 政黨：{}", words[1], words[0]);
                    }
                    _ => println!("輸入格式錯誤"),
                }
            } else {
                println!("請輸入範圍內的正整數");
            }
        } else {
            println!("請輸入範圍內的正整數");
        }
    }
    Ok(())
}
