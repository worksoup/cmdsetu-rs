use mirai_j4rs::{
    contact::bot::{BotConfiguration, Certificate, Env},
    other::enums::MiraiProtocol,
};
use serde::Deserialize;
use std::{fs, path::PathBuf};
use toml::Table;

#[tokio::main]
async fn main() {
    #[derive(Deserialize)]
    struct TmpEnv {
        core_path: String,
        java_opt: String,
    }
    #[derive(Deserialize)]
    struct QqUser {
        bot_id: i64,
        bot_passwd: String,
    }
    #[derive(Deserialize)]
    struct UserConfig {
        env: TmpEnv,
        bot: QqUser,
    }
    let user_config = fs::read_to_string("./config.toml").unwrap();
    let user_config: UserConfig = toml::from_str(user_config.as_str()).unwrap();
    let env: Env = Env::new_env(
        PathBuf::from(user_config.env.core_path.clone()),
        &user_config.env.java_opt,
    );
    env.fix_protocol_version_fetch(MiraiProtocol::A, "latest".to_owned());
    let bot_config = BotConfiguration::get_default();
    bot_config.default_device_info_file();
    let mut bot = env.new_bot(
        user_config.bot.bot_id,
        user_config.bot.bot_passwd.as_str(),
        bot_config.into(),
    );
    bot.login();
}
