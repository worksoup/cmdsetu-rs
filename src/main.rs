#![feature(atomic_bool_fetch_not)]
#![feature(async_closure)]
#![feature(let_chains)]
#![feature(lint_reasons)]

mod prelude;

use prelude::*;

use extra_test::ExtraTest;
use futures::{
    future::join_all,
    stream::{FuturesUnordered, StreamExt},
};
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use std::{collections::HashMap, fs, path::PathBuf};
use strfmt::strfmt;
use tokio::{io::AsyncWriteExt, join, select, sync::Mutex};
lazy_static! {
    static ref CONFIG: Config =
        toml::from_str(fs::read_to_string("./config.toml").unwrap().as_str()).unwrap();
}
fn determine_auth(bot: &BotInfo) -> BotAuthorization {
    fn parse_md5(md5_str: &str) -> [u8; 16] {
        let md5_vec = md5_str
            .chars()
            .collect::<Vec<_>>()
            .chunks(2)
            .map(|c| {
                let str = c.iter().collect::<String>();
                u8::from_str_radix(str.as_str(), 16).expect("MD5无法解析为字节数组！")
            })
            .collect::<Vec<u8>>();
        let mut md5 = [0; 16];
        for i in 0..16 {
            md5[i] = md5_vec[i];
        }
        md5
    }
    fn auto(bot: &BotInfo) -> BotAuthorization {
        if !bot.bot_passwd.is_empty() {
            println!("--- BotAuthorization::Password");
            BotAuthorization::Password(bot.bot_passwd.clone())
        } else if !bot.bot_passwd_md5.is_empty() {
            println!("--- BotAuthorization::Md5");
            let md5 = parse_md5(&bot.bot_passwd_md5);
            BotAuthorization::Md5(md5)
        } else {
            println!("--- BotAuthorization::QrCode");
            BotAuthorization::QrCode
        }
    }
    if bot.auth == "QRCODE" {
        println!("--- BotAuthorization::QrCode");
        BotAuthorization::QrCode
    } else if bot.auth == "PASSWORD" {
        println!("--- BotAuthorization::Password");
        BotAuthorization::Password(bot.bot_passwd.clone())
    } else if bot.auth == "MD5" {
        println!("--- BotAuthorization::Md5");
        let md5 = parse_md5(&bot.bot_passwd_md5);
        BotAuthorization::Md5(md5)
    } else if bot.auth == "AUTO" {
        auto(bot)
    } else {
        eprintln!("无法解析登录方式，将根据其他配置确定！");
        auto(bot)
    }
}
fn determine_protocol(protocol: &str) -> MiraiProtocol {
    match protocol {
        "A" => MiraiProtocol::A,
        "I" => MiraiProtocol::I,
        "M" => MiraiProtocol::M,
        "P" => MiraiProtocol::P,
        "W" => MiraiProtocol::W,
        _ => {
            eprintln!("协议枚举转换失败，默认转换结果为安卓协议。");
            MiraiProtocol::A
        }
    }
}
#[tokio::main]
async fn main() {
    let (ql_tx, mut ql_rx) = futures::channel::mpsc::unbounded();
    let (lq_tx, mut lq_rx) = futures::channel::mpsc::unbounded::<(
        Group,
        Member,
        Mutex<HashMap<PathBuf, MessageChain>>,
    )>();
    let (ctrlc_tx, mut ctrlc_rx) = futures::channel::mpsc::unbounded();
    let ql_tx = Box::leak(Box::new(ql_tx));
    let rx = Box::leak(Box::new(Regex::new(&CONFIG.cmn_rx).unwrap()));
    println!(
        "-- {:?}, \n-- {:?}",
        CONFIG.prem.groups, CONFIG.prem.members
    );
    let bot_authorization = determine_auth(&CONFIG.bot);
    let protocol = determine_protocol(&*CONFIG.bot.protocol);
    let extra = move |jvm: &j4rs::Jvm, b1: &j4rs::Instance, b2: &j4rs::Instance| match protocol {
        MiraiProtocol::A | MiraiProtocol::P => ExtraTest::load(protocol)(jvm, b1, b2),
        _ => {}
    };
    let bot = BotBuilder::create(".", &CONFIG.jvm.jars, &CONFIG.jvm.opts)
        .extra(extra)
        .id(CONFIG.bot.bot_id)
        .authorization(bot_authorization)
        .file_based_device_info(None)
        .protocol(determine_protocol(CONFIG.bot.protocol.as_str()))
        .build();
    bot.login();
    let event_channel = bot.get_event_channel();
    let on_group_message_event: Box<dyn Fn(GroupMessageEvent)> =
        Box::new(|event: GroupMessageEvent| {
            let group = event.get_subject();
            let sender = event.get_sender();
            if CONFIG.prem.groups.contains(&group.get_id())
                && CONFIG.prem.members.contains(&sender.get_id())
            {
                let msg = event.get_message().to_content();
                let caps = rx.captures(&msg);
                if let Some(caps) = caps {
                    match rxcap(caps) {
                        Ok((num, r18, tag, ai)) => {
                            let req_data = build_req_data(num, r18, tag, ai, &group, &CONFIG);
                            let _ = ql_tx.unbounded_send((group, sender, req_data));
                        }
                        Err(err) => {
                            handle_err(err, &group, &CONFIG);
                        }
                    }
                }
            }
        });
    let listener_for_group_message_event = event_channel.subscribe_always(&on_group_message_event);
    let send_image_task = async {
        while let Some((
            group,
            _sender, //私发功能要用来着，但是懒得写了，又不是不能用。
            msgs_m,
        )) = lq_rx.next().await
        {
            for (filepath, msg) in &*msgs_m.lock().await {
                if let Ok(_) = filepath.metadata() {
                    let image = group.upload_image_from_file(filepath.to_str().unwrap());
                    group.send_message(msg.plus(image));
                } else {
                    let bad_msg = PlainText::from(CONFIG.err_msg.bad_dld.clone());
                    group.send_message(msg.plus(bad_msg));
                }
            }
            // 发送完毕。
            group.send_string(&CONFIG.tip_msg.tip_end);
        }
    };
    let tasks = Mutex::new(FuturesUnordered::new());
    let download_task = async {
        let client: Client = Client::new();
        while let Some(data) = ql_rx.next().await {
            println!("{:?}", data.2);
            use trauma::download::Download;
            use trauma::downloader::DownloaderBuilder;
            use url::Url;
            let mut pic_dir = std::env::current_dir().unwrap();
            pic_dir.push("pictures");
            let downloader = DownloaderBuilder::new().directory(pic_dir).build();
            let send_post = client.post(&CONFIG.api_url).json(&data.2).send();
            let lq_tx = lq_tx.clone();
            // task 干的事情：
            //      发送 post 请求。
            //      获取响应数据然后异步地下载图片和构造不包含图片的 MessageChain.
            let task = async move || {
                match send_post.await {
                    Ok(resq) => {
                        if let Ok(resq_data) = resq.json::<RespData>().await {
                            if resq_data.error.is_empty() {
                                let resq_data_len = resq_data.data.len();
                                // println!("响应图片数量：{}", resq_data_len);
                                if resq_data_len > 0 {
                                    if resq_data_len < data.2.num.into() {
                                        let n = {
                                            let mut n = HashMap::new();
                                            n.insert("n".to_string(), resq_data_len.to_string());
                                            n
                                        };
                                        // 请求的数量小于返回的数量。
                                        data.0.send_string(
                                            &strfmt(&CONFIG.err_msg.bad_eql, &n).unwrap(),
                                        );
                                    }
                                    let mut downloads = Vec::new();
                                    let pic_meta_path = {
                                        let mut tmp_path = std::env::current_dir().unwrap();
                                        tmp_path.push("pictures");
                                        tmp_path.push("metadata");
                                        tmp_path
                                    };
                                    let map = Mutex::new(HashMap::new());
                                    let mut jobs = Vec::new();
                                    for pic_data in &resq_data.data {
                                        let url = Url::parse(&pic_data.urls.original).unwrap();
                                        let filename = url
                                            .path_segments()
                                            .unwrap()
                                            .last()
                                            .unwrap()
                                            .to_string();
                                        let pic_path = {
                                            let mut tmp_path = std::env::current_dir().unwrap();
                                            tmp_path.push("pictures");
                                            tmp_path.push(&filename);
                                            tmp_path
                                        };
                                        if let Err(_) = fs::metadata(&pic_path) {
                                            downloads.push(Download::new(&url, &filename));
                                        } else if rand::thread_rng().gen_range(0..=20) > 3 {
                                            downloads.push(Download::new(&url, &filename));
                                        }
                                        if let Err(_) = fs::metadata(&pic_meta_path) {
                                            let _ = fs::create_dir_all(&pic_meta_path);
                                        }
                                        let job = async {
                                            let data_toml = toml::to_string(pic_data).unwrap();
                                            let mut path = pic_meta_path.clone();
                                            path.push(filename + ".toml");
                                            let mut file =
                                                tokio::fs::File::create(&path).await.unwrap();
                                            file.write_all(data_toml.as_bytes()).await.unwrap();
                                            let at = At::new(data.1.get_id());
                                            let tip_doc = {
                                                let mut tip_doc = HashMap::new();
                                                tip_doc.insert(
                                                    "title".to_string(),
                                                    pic_data.title.clone(),
                                                );
                                                tip_doc.insert(
                                                    "pid".to_string(),
                                                    pic_data.pid.to_string(),
                                                );
                                                tip_doc.insert(
                                                    "author".to_string(),
                                                    pic_data.author.clone(),
                                                );
                                                tip_doc.insert(
                                                    "uid".to_string(),
                                                    pic_data.uid.to_string(),
                                                );
                                                tip_doc.insert(
                                                    "tags".to_string(),
                                                    std::format!("{:?}", pic_data.tags),
                                                );
                                                tip_doc.insert("is_Ai".to_string(), {
                                                    match pic_data.aiType {
                                                        1 => "否".to_string(),
                                                        2 => "是".to_string(),
                                                        _ => "存疑".to_string(),
                                                    }
                                                });
                                                tip_doc
                                            };
                                            let msgs_m = at.plus(PlainText::from(
                                                strfmt(&CONFIG.tip_msg.tip_doc, &tip_doc).unwrap(),
                                            ));
                                            map.lock().await.insert(pic_path, msgs_m);
                                        };
                                        jobs.push(job);
                                    }
                                    // for tmp in &downloads {
                                    //     println!("下载内容：{:?}", tmp);
                                    // }
                                    join!(join_all(jobs), downloader.download(&downloads));
                                    let _ = lq_tx.unbounded_send((data.0, data.1, map));
                                } else {
                                    // 没有响应的数据。
                                    data.0.send_string(&CONFIG.err_msg.bad_url.clone());
                                }
                            } else {
                                let bad_rsp_msg = {
                                    let mut tmp = HashMap::new();
                                    tmp.insert("msg".to_string(), resq_data.error.clone());
                                    tmp
                                };
                                // 响应失败。
                                data.0.send_string(
                                    &strfmt(&CONFIG.err_msg.bad_rsp.clone(), &bad_rsp_msg).unwrap(),
                                );
                            }
                        }
                    }
                    Err(_) => {
                        // 请求失败。
                        data.0.send_string(&CONFIG.err_msg.bad_req.clone());
                    }
                };
            };
            let tasks = tasks.lock().await;
            tasks.push(task());
        }
    };
    let forward_task = async {
        loop {
            let mut tasks = tasks.lock().await;
            if let Some(_) = tasks.next().await {}
        }
    };
    let ctrlc_task = async {
        while let Some(_) = ctrlc_rx.next().await {
            break;
        }
    };
    ctrlc::set_handler(move || {
        ctrlc_tx.unbounded_send(()).unwrap();
    })
    .unwrap();
    select! {
        _ = send_image_task =>{},
        _ = download_task => {},
        _ = forward_task => {},
        _ = ctrlc_task => {}
    }
    listener_for_group_message_event.complete();
    println!("complete!");
}
