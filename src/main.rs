#![feature(atomic_bool_fetch_not)]
#![feature(async_closure)]
use chinese_number::{ChineseCountMethod, ChineseToNumber, ChineseToNumberError};
use futures::{
    executor::block_on,
    future::{join, join_all},
    stream::{FuturesUnordered, StreamExt},
};
use lazy_static::lazy_static;
use mirai_j4rs::{
    contact::{
        bot::{BotConfiguration, Certificate, Env},
        contact_trait::{ContactOrBotTrait, ContactTrait},
        group::Group,
    },
    event::message::{GroupMessageEvent, MessageEventTrait},
    message::{message_trait::MessageTrait, At, MessageChain, PlainText},
    other::enums::MiraiProtocol,
};
use rand::Rng;
use regex::{Match, Regex};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell, collections::HashMap, error::Error, fs, num::ParseIntError, path::PathBuf,
    sync::Arc,
};
use tokio::{io::AsyncWriteExt, join, sync::Mutex};

#[derive(Deserialize, Serialize)]
struct EnvConfig {
    core_path: String,
    java_opt: String,
}
#[derive(Deserialize, Serialize)]
struct BotInfo {
    bot_id: i64,
    bot_passwd: String,
}
#[derive(Deserialize, Serialize)]
struct PremInfo {
    groups: Vec<i64>,
    members: Vec<i64>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
struct ReqData {
    r18: u8,
    num: u8,
    uid: Vec<i64>,
    keyword: String,
    tag: Vec<String>,
    size: Vec<String>,
    proxy: String,
    dateAfter: i64,
    dateBefor: i64,
    dsc: bool,
    excludeAI: bool,
}
#[derive(Deserialize, Serialize)]
struct PixUrl {
    original: String,
}
#[derive(Deserialize, Serialize)]
struct PicData {
    pid: i64,
    p: i64,
    title: String,
    author: String,
    r18: bool,
    width: i64,
    height: i64,
    tags: Vec<String>,
    ext: String,
    aiType: i8,
    uploadDate: i64,
    urls: PixUrl,
}
#[derive(Deserialize, Serialize)]
struct RespData {
    error: String,
    data: Vec<PicData>,
}
#[derive(Deserialize, Serialize)]
struct ErrMsg {
    bad_url: String,
    bad_resp: String,
    bad_req: String,
}
#[derive(Deserialize, Serialize)]
struct Config {
    api_url: String,
    cmn_rx: String,
    env: EnvConfig,
    bot: BotInfo,
    prem: PremInfo,
    default_req: ReqData,
    err_msg: ErrMsg,
}
fn zh2num(s: &str) -> Result<i128, ChineseToNumberError> {
    let mut chars_mapping: HashMap<char, char> = HashMap::new();
    chars_mapping.insert('〇', '零');
    chars_mapping.insert('一', '壹');
    chars_mapping.insert('弌', '壹');
    chars_mapping.insert('二', '贰');
    chars_mapping.insert('貳', '贰');
    chars_mapping.insert('两', '贰');
    chars_mapping.insert('兩', '贰');
    chars_mapping.insert('弐', '贰');
    chars_mapping.insert('三', '叁');
    chars_mapping.insert('参', '叁');
    chars_mapping.insert('參', '叁');
    chars_mapping.insert('弎', '叁');
    chars_mapping.insert('亖', '肆');
    chars_mapping.insert('四', '肆');
    chars_mapping.insert('五', '伍');
    chars_mapping.insert('六', '陆');
    chars_mapping.insert('陸', '陆');
    chars_mapping.insert('七', '柒');
    chars_mapping.insert('八', '捌');
    chars_mapping.insert('九', '玖');
    chars_mapping.insert('十', '拾');
    chars_mapping.insert('什', '拾');
    chars_mapping.insert('百', '佰');
    chars_mapping.insert('陌', '佰');
    chars_mapping.insert('千', '仟');
    chars_mapping.insert('阡', '仟');
    chars_mapping.insert('萬', '万');
    chars_mapping.insert('億', '亿');
    chars_mapping.insert('壤', '穰');
    chars_mapping.insert('溝', '沟');
    chars_mapping.insert('澗', '涧');
    chars_mapping.insert('載', '载');
    chars_mapping.insert('極', '极');
    let mut ir = String::new();
    for c in s.chars() {
        if let Some(&r) = chars_mapping.get(&c) {
            ir.push(r);
        } else {
            match c {
                '念' | '卄' | '廿' => ir.push_str("贰拾"),
                '卅' => ir.push_str("叁拾"),
                '卌' => ir.push_str("肆拾"),
                '皕' => ir.push_str("贰佰"),
                _ => ir.push(c),
            }
        }
    }
    ir.to_number(ChineseCountMethod::High)
}
fn rxcap(caps: Option<regex::Captures<'_>>) -> Result<(i128, u8, Vec<String>), Box<dyn Error>> {
    println!("caps: {:?}", caps);
    let mut pic_count = 1;
    let mut r18 = 0; // 默认是非 r18 模式。
    let mut tags = Vec::new();
    if let Some(cap) = caps {
        if let Some(hans_num) = cap.name("hans_num") {
            pic_count = zh2num(hans_num.as_str())?;
        } else if let Some(comn_num) = cap.name("comn_num") {
            pic_count = comn_num.as_str().parse()?;
        } else if let Some(weak_num) = cap.name("weak_num") {
            match weak_num.as_str() {
                "俩仨" => pic_count = rand::thread_rng().gen_range(1..=4),
                "俩" => pic_count = 2,
                "仨" => pic_count = 3,
                _ => (),
            }
        } else if let Some(_) = cap.name("more") {
            pic_count = rand::thread_rng().gen_range(5..=10);
        } else if let Some(_) = cap.name("less") {
            pic_count = rand::thread_rng().gen_range(1..=4);
        }
        if let Some(nsfw) = cap.name("nsfw") {
            if !nsfw.is_empty() {
                r18 = 1;
            }
        }
        tags = get_tags(cap.name("tags"));

        println!("{:?}", cap.name("hans_num"));
        println!("{:?}", cap.name("comn_num"));
        println!("{:?}", cap.name("weak_num"));
        println!("{:?}", cap.name("more"));
        println!("{:?}", cap.name("less"));
        println!("{:?}", cap.name("nsfw"));
        for tag in get_tags(cap.name("tags")) {
            println!("{:?}", tag);
        }
    }
    Ok((pic_count, r18, tags))
}
fn get_tags(cap: Option<Match<'_>>) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(tags_) = cap {
        let mut begin: usize = 0;
        let mut current: usize = 0;
        let mut quote_begin: Option<usize> = None;
        for c in tags_.as_str().chars() {
            match c {
                '“' => {
                    quote_begin = Some(current);
                }
                '”' => {
                    if let Some(quote_begin) = quote_begin {
                        if quote_begin + 1 < current - 1 {
                            tags.push(
                                utf8_slice::slice(&tags_.as_str(), quote_begin + 1, current)
                                    .to_owned(),
                            );
                        }
                    };
                    quote_begin = None;
                    begin = current + 1;
                }
                '的' => {
                    if quote_begin.is_none() {
                        if current != begin {
                            tags.push(
                                utf8_slice::slice(&tags_.as_str(), begin, current).to_owned(),
                            );
                        }
                        begin = current + 1;
                    }
                }
                _ => {}
            }
            current += 1;
        }
    }
    tags
}
lazy_static! {
    static ref CONFIG: Config =
        toml::from_str(fs::read_to_string("./config.toml").unwrap().as_str()).unwrap();
}
#[tokio::main]
async fn main() {
    let (ql_tx, mut ql_rx) = futures::channel::mpsc::unbounded();
    let (lq_tx, mut lq_rx) =
        futures::channel::mpsc::unbounded::<(Group, Mutex<HashMap<PathBuf, MessageChain>>)>();
    let ql_tx = Box::leak(Box::new(ql_tx));
    let env: Env = Env::new_env(
        PathBuf::from(CONFIG.env.core_path.clone()),
        &CONFIG.env.java_opt,
    );
    let rx = Box::leak(Box::new(Regex::new(&CONFIG.cmn_rx).unwrap()));
    println!(
        "-- {:?}, \n-- {:?}",
        CONFIG.prem.groups, CONFIG.prem.members
    );
    env.fix_protocol_version_fetch(MiraiProtocol::A, "latest".to_owned());
    let bot_config = BotConfiguration::get_default();
    bot_config.default_device_info_file();
    let mut bot = env.new_bot(
        CONFIG.bot.bot_id,
        CONFIG.bot.bot_passwd.as_str(),
        bot_config.into(),
    );
    bot.login();
    let event_channel = bot.get_event_channel();
    let callback: Box<dyn Fn(GroupMessageEvent)> = Box::new(|event: GroupMessageEvent| {
        let group = event.get_subject();
        let sender_id = event.get_sender().get_id();
        if CONFIG.prem.groups.contains(&group.get_id()) && CONFIG.prem.members.contains(&sender_id)
        {
            let msg = event.get_message().to_content_text();
            println!("获取到消息：{}", msg);
            let caps = rx.captures(&msg);
            match rxcap(caps) {
                Ok((num, r18, tag)) => {
                    group.send_string(
                        (String::from("正在获取，数量：") + num.to_string().as_str() + "张……")
                            .as_str(),
                    );
                    let mut req_data = CONFIG.default_req.clone();
                    req_data.r18 = r18;
                    if num > 20 {
                        req_data.num = rand::thread_rng().gen_range(1..=20);
                    } else {
                        req_data.num = if num == 0 { 1 } else { num as u8 };
                    }
                    req_data.tag = tag;
                    let _ = ql_tx.unbounded_send((group, sender_id, req_data));
                }
                Err(err) => {
                    if let Some(err) = err.downcast_ref::<ChineseToNumberError>() {
                        match err {
                            ChineseToNumberError::ChineseNumberIncorrect { char_index } => {
                                todo!("数字格式不正确。");
                            }
                            ChineseToNumberError::Overflow | ChineseToNumberError::Underflow => {
                                todo!("数据溢出。");
                            }
                            _ => todo!("意料之外的错误。"),
                        }
                    } else if let Some(err) = err.downcast_ref::<ParseIntError>() {
                        match err.kind() {
                            std::num::IntErrorKind::PosOverflow
                            | std::num::IntErrorKind::NegOverflow => {
                                todo!("数据溢出。");
                            }
                            _ => panic!("意料之外的错误。"),
                        }
                    }
                }
            }
        }
    });
    let listener = event_channel.subscribe_always(&callback);
    let f1 = async {
        while let Some((group, msgs_m)) = lq_rx.next().await {
            for (filepath, msg) in &*msgs_m.lock().await {
                let image = group.upload_image_from_file(filepath);
                group.send_message(msg.plus(image));
            }
        }
    };
    let tasks = Mutex::new(FuturesUnordered::new());
    let f2 = async {
        let client: Client = reqwest::Client::new();
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
                            // struct PicData {
                            //     pid: i64,
                            //     p: i64,
                            //     title: String,
                            //     author: String,
                            //     r18: bool,
                            //     width: i64,
                            //     height: i64,
                            //     tags: Vec<String>,
                            //     ext: String,
                            //     aiType: i8,
                            //     uploadDate: i64,
                            //     urls: PixUrl,
                            // }
                            let resq_data_len = resq_data.data.len();
                            println!("响应图片数量：{}", resq_data_len);
                            if resq_data_len > 0 {
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
                                    let filename =
                                        url.path_segments().unwrap().last().unwrap().to_string();
                                    let pic_path = {
                                        let mut tmp_path = std::env::current_dir().unwrap();
                                        tmp_path.push("pictures");
                                        tmp_path.push(&filename);
                                        tmp_path
                                    };
                                    if let Err(_) = std::fs::metadata(&pic_path) {
                                        downloads.push(Download::new(&url, &filename));
                                    } else if rand::thread_rng().gen_range(0..=20) > 3 {
                                        downloads.push(Download::new(&url, &filename));
                                    }
                                    if let Err(_) = std::fs::metadata(&pic_meta_path) {
                                        let _ = std::fs::create_dir_all(&pic_meta_path);
                                    }
                                    let job = async {
                                        let data_toml = toml::to_string(pic_data).unwrap();
                                        let mut path = pic_meta_path.clone();
                                        path.push(filename + ".toml");
                                        let mut file =
                                            tokio::fs::File::create(&path).await.unwrap();
                                        file.write_all(data_toml.as_bytes()).await.unwrap();
                                        let at = At::new(data.1);
                                        let msgs_m = at.plus(PlainText::from(format!(""))); // TODO
                                        map.lock().await.insert(pic_path, msgs_m);
                                    };
                                    jobs.push(job);
                                }
                                for tmp in &downloads {
                                    println!("下载内容：{:?}", tmp);
                                }
                                join!(join_all(jobs), downloader.download(&downloads));
                                let _ = lq_tx.unbounded_send((data.0, map));
                            } else {
                                todo!("检查xp");
                            }
                        }
                    }
                    Err(err) => println!("{}", err),
                };
            };
            let tasks = tasks.lock().await;
            tasks.push(task());
        }
    };
    let f3 = async {
        loop {
            let mut tasks = tasks.lock().await;
            if let Some(_) = tasks.next().await {}
        }
    };
    join!(f1, f2, f3);
    listener.complete();
    println!("complete!");
}
