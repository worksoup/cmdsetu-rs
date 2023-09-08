use std::{collections::HashMap, error::Error, num::ParseIntError};

use super::structs::{Config, ReqData};
use chinese_number::{ChineseCountMethod, ChineseToNumber, ChineseToNumberError};
use mirai_j4rs::contact::{contact_trait::ContactTrait, group::Group};
use rand::Rng;
use regex::Match;
use strfmt::strfmt;

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
    if let Some(hans_num) = cap.name("hans_num") && !hans_num.is_empty() {
        pic_count = zh2num(hans_num.as_str())?;
    } else if let Some(comn_num) = cap.name("comn_num")  && !comn_num.is_empty() {
        pic_count = comn_num.as_str().parse()?;
    } else if let Some(weak_num) = cap.name("weak_num") && !weak_num.is_empty() {
        match weak_num.as_str() {
            "俩仨" => pic_count = rand::thread_rng().gen_range(1..=4),
            "俩" => pic_count = 2,
            "仨" => pic_count = 3,
            _ => (),
        }
    } else if let Some(more) = cap.name("more") && !more.is_empty() {
        pic_count = rand::thread_rng().gen_range(5..=10);
    } else if let Some(less) = cap.name("less") && !less.is_empty() {
        pic_count = rand::thread_rng().gen_range(1..=4);
    }
    if let Some(nsfw) = cap.name("nsfw") && !nsfw.is_empty() {
        r18 = 1;
    }
    if let Some(ai_tmp) = cap.name("ai") && !ai_tmp.is_empty() {
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
