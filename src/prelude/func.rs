use std::{collections::HashMap, error::Error, fs, num::ParseIntError, path::PathBuf};

use super::structs::{Config, ReqData};
use crate::{prelude::*, CONFIG};
use chinese_number::{ChineseCountMethod, ChineseToNumber, ChineseToNumberError};
use futures::{channel::mpsc::UnboundedSender, future::join_all};
use rand::Rng;
use regex::Match;
use reqwest::Response;
use strfmt::strfmt;
use tokio::{io::AsyncWriteExt, join, sync::Mutex};
use trauma::{download::Download, downloader::Downloader};
use url::Url;

pub(crate) fn zh2num(s: &str) -> Result<i128, ChineseToNumberError> {
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

pub(crate) fn rxcap(
    cap: regex::Captures<'_>,
) -> Result<(i128, u8, Vec<String>, bool), Box<dyn Error>> {
    let mut pic_count = 1;
    let mut r18 = 0; // 默认是非 r18 模式。
    let tags;
    let mut ai = true;
    if let Some(hans_num) = cap.name("hans_num")
        && !hans_num.is_empty()
    {
        pic_count = zh2num(hans_num.as_str())?;
    } else if let Some(comn_num) = cap.name("comn_num")
        && !comn_num.is_empty()
    {
        pic_count = comn_num.as_str().parse()?;
    } else if let Some(weak_num) = cap.name("weak_num")
        && !weak_num.is_empty()
    {
        match weak_num.as_str() {
            "俩仨" => pic_count = rand::thread_rng().gen_range(1..=4),
            "俩" => pic_count = 2,
            "仨" => pic_count = 3,
            _ => (),
        }
    } else if let Some(more) = cap.name("more")
        && !more.is_empty()
    {
        pic_count = rand::thread_rng().gen_range(5..=10);
    } else if let Some(less) = cap.name("less")
        && !less.is_empty()
    {
        pic_count = rand::thread_rng().gen_range(1..=4);
    }
    if let Some(nsfw) = cap.name("nsfw")
        && !nsfw.is_empty()
    {
        r18 = 1;
    }
    if let Some(ai_tmp) = cap.name("ai")
        && !ai_tmp.is_empty()
    {
        ai = false;
    }
    tags = get_tags(cap.name("tags"));

    println!("{:?}", cap.name("hans_num"));
    println!("{:?}", cap.name("comn_num"));
    println!("{:?}", cap.name("weak_num"));
    println!("{:?}", cap.name("more"));
    println!("{:?}", cap.name("less"));
    println!("{:?}", cap.name("nsfw"));
    for tag in get_tags(cap.name("tags")) {
        println!("----tag----");
        println!("{:?}", tag);
    }

    Ok((pic_count, r18, tags, ai))
}

// 该函数不返回 None
pub(crate) fn get_tags(cap: Option<Match<'_>>) -> Vec<String> {
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

pub(crate) fn build_req_data(
    num: i128,
    r18: u8,
    tag: Vec<String>,
    ai: bool,
    group: &Group,
    config: &Config,
) -> ReqData {
    let n = {
        let mut n = HashMap::new();
        n.insert("n".to_string(), num.to_string());
        n
    };
    // DEFAULT -- tip_cmd = "收到指令：获取{n}张色图。正在处理中……"
    group.send_string(&strfmt(&config.tip_msg.tip_cmd, &n).unwrap());
    let mut req_data = config.default_req.clone();
    req_data.r18 = r18;
    if num > 9_4266 {
        req_data.num = rand::thread_rng().gen_range(1..=20);
        let n = {
            let mut n = HashMap::new();
            n.insert("n".to_string(), req_data.num.to_string());
            n
        };
        // 请求的数量超过了数据库总量。
        group.send_string(&strfmt(&config.err_msg.bad_hug, &n).unwrap());
    } else if num > 20 {
        req_data.num = rand::thread_rng().gen_range(1..=20);
        let n: HashMap<String, String> = {
            let mut n = HashMap::new();
            n.insert("n".to_string(), req_data.num.to_string());
            n
        };
        // 请求的数字超过 api 限制。
        group.send_string(&strfmt(&config.err_msg.bad_lim, &n).unwrap());
    } else {
        req_data.num = if num == 0 { 1 } else { num as u8 };
    }
    req_data.tag = tag;
    req_data.excludeAI = ai;
    req_data
}

pub(crate) fn handle_err(err: Box<dyn Error>, group: &Group, config: &Config) {
    if let Some(err) = err.downcast_ref::<ChineseToNumberError>() {
        match err {
            ChineseToNumberError::ChineseNumberIncorrect { char_index } => {
                let n = {
                    let mut n = HashMap::new();
                    n.insert("n".to_string(), char_index.to_string());
                    n
                };
                // 数字格式不正确。
                group.send_string(&strfmt(&config.err_msg.bad_num, &n).unwrap());
            }
            ChineseToNumberError::Overflow | ChineseToNumberError::Underflow => {
                // 数据溢出。
                group.send_string(&&config.err_msg.bad_int);
            }
            _ => {
                // 意料之外的错误。
                group.send_string(&config.err_msg.bad_bad);
                println!("{}", err);
            }
        }
    } else if let Some(err) = err.downcast_ref::<ParseIntError>() {
        match err.kind() {
            std::num::IntErrorKind::PosOverflow | std::num::IntErrorKind::NegOverflow => {
                group.send_string(&&config.err_msg.bad_int);
            }
            _ => {
                // 意料之外的错误。
                group.send_string(&config.err_msg.bad_bad);
                println!("{}", err);
            }
        }
    }
}

// task 干的事情：
//      发送 post 请求。
//      获取响应数据然后异步地下载图片和构造不包含图片的 MessageChain.
pub(crate) async fn task(
    lq_tx: UnboundedSender<(Group, Member, Mutex<HashMap<PathBuf, MessageChain>>)>,
    downloader: Downloader,
    group: Group,
    member: Member,
    req_data: ReqData,
    send_post: Result<Response, reqwest::Error>,
) {
    if send_post.is_err() {
        // 请求失败。
        group.send_string(&CONFIG.err_msg.bad_req.clone());
        return;
    }
    let resq_data: RespData = send_post.unwrap().json().await.unwrap();
    let error = resq_data.error;
    if !error.is_empty() {
        let mut tmp = HashMap::new();
        tmp.insert("msg".to_string(), error.clone());
        // 响应失败。
        group.send_string(&strfmt(&CONFIG.err_msg.bad_rsp.clone(), &tmp).unwrap());
        return;
    }
    let data = resq_data.data;
    // println!("响应图片数量：{}", resq_data_len);
    if data.len() == 0 {
        // 没有响应的数据。
        group.send_string(&CONFIG.err_msg.bad_url.clone());
        return;
    }
    if data.len() < req_data.num.into() {
        let mut tmp = HashMap::new();
        tmp.insert("n".to_string(), data.len().to_string());
        // 请求的数量小于返回的数量。
        group.send_string(&strfmt(&CONFIG.err_msg.bad_eql, &tmp).unwrap());
    }

    let mut pic_meta_path = std::env::current_dir().unwrap();
    pic_meta_path.push("pictures");
    pic_meta_path.push("metadata");

    let map = Mutex::new(HashMap::new());

    let mut jobs = Vec::new();
    let mut downloads = Vec::new();

    for pic_data in &data {
        let url = Url::parse(&pic_data.urls.original).unwrap();
        let filename = url.path_segments().unwrap().last().unwrap().to_string();

        let mut pic_path = std::env::current_dir().unwrap();
        pic_path.push("pictures");
        pic_path.push(&filename);

        if fs::metadata(&pic_path).is_err() {
            downloads.push(Download::new(&url, &filename));
        } else if rand::thread_rng().gen_range(0..=20) > 3 {
            downloads.push(Download::new(&url, &filename));
        }
        if fs::metadata(&pic_meta_path).is_err() {
            let _ = fs::create_dir_all(&pic_meta_path);
        }
        let job = async {
            let data_toml = toml::to_string(pic_data).unwrap();
            let mut path = pic_meta_path.clone();
            path.push(filename + ".toml");
            let mut file = tokio::fs::File::create(&path).await.unwrap();
            file.write_all(data_toml.as_bytes()).await.unwrap();
            let tip_doc = {
                let mut tip_doc = HashMap::new();
                tip_doc.insert("title".to_string(), pic_data.title.clone());
                tip_doc.insert("pid".to_string(), pic_data.pid.to_string());
                tip_doc.insert("author".to_string(), pic_data.author.clone());
                tip_doc.insert("uid".to_string(), pic_data.uid.to_string());
                tip_doc.insert("tags".to_string(), std::format!("{:?}", pic_data.tags));
                tip_doc.insert("is_Ai".to_string(), {
                    match pic_data.aiType {
                        1 => "否".to_string(),
                        2 => "是".to_string(),
                        _ => "存疑".to_string(),
                    }
                });
                tip_doc
            };
            let msgs_m = At::new(member.get_id()).plus(PlainText::from(
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
    let _ = lq_tx.unbounded_send((group, member, map));
}
