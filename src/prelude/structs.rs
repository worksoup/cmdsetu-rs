use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct BotInfo {
    pub(crate) bot_id: i64,
    pub(crate) bot_passwd: String,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct PremInfo {
    pub(crate) groups: Vec<i64>,
    pub(crate) members: Vec<i64>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
#[allow(
    non_snake_case,
    reason = "这是一个要求序列化为 json 的结构体，post 时要求的 json 数据格式没有采用蛇形命名法。"
)]
pub(crate) struct ReqData {
    pub(crate) r18: u8,
    pub(crate) num: u8,
    pub(crate) uid: Vec<i64>,
    pub(crate) keyword: String,
    pub(crate) tag: Vec<String>,
    pub(crate) size: Vec<String>,
    pub(crate) proxy: String,
    pub(crate) dateAfter: i64,
    pub(crate) dateBefore: i64,
    pub(crate) dsc: bool,
    pub(crate) excludeAI: bool,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct PixUrl {
    pub(crate) original: String,
}
#[allow(
    non_snake_case,
    reason = "这是一个要求序列化为 json 的结构体，post 时要求的 json 数据格式没有采用蛇形命名法。"
)]
#[derive(Deserialize, Serialize)]
pub(crate) struct PicData {
    pub(crate) pid: i64,
    pub(crate) p: i64,
    pub(crate) uid: i64,
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) r18: bool,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) tags: Vec<String>,
    pub(crate) ext: String,
    pub(crate) aiType: i8,
    pub(crate) uploadDate: i64,
    pub(crate) urls: PixUrl,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct RespData {
    pub(crate) error: String,
    pub(crate) data: Vec<PicData>,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct ErrMsg {
    pub(crate) bad_url: String,
    pub(crate) bad_rsp: String,
    pub(crate) bad_req: String,
    pub(crate) bad_dld: String,
    pub(crate) bad_int: String,
    pub(crate) bad_num: String,
    pub(crate) bad_lim: String,
    pub(crate) bad_bad: String,
    pub(crate) bad_hug: String,
    pub(crate) bad_eql: String,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct TipMsg {
    pub(crate) tip_cmd: String,
    pub(crate) tip_doc: String,
    pub(crate) tip_end: String,
}
#[derive(Deserialize, Serialize)]
pub(crate) struct Config {
    pub(crate) api_url: String,
    pub(crate) cmn_rx: String,
    pub(crate) bot: BotInfo,
    pub(crate) prem: PremInfo,
    pub(crate) default_req: ReqData,
    pub(crate) err_msg: ErrMsg,
    pub(crate) tip_msg: TipMsg,
}
