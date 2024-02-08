pub(crate) use mirai_j4rs::{
    auth::bot_authorization::BotAuthorization,
    contact::{ContactOrBotTrait, ContactTrait, Group, Member, SendMessageSupportedTrait},
    event::{FriendMessageEvent, GroupMessageEvent, MessageEventTrait},
    message::{
        data::{
            At, Audio, MarketFaceAll, MessageChain, PlainText, RockPaperScissors, SingleMessage,
        },
        MarketFaceTrait, MessageTrait,
    },
    mj_base::env::GetInstanceTrait,
    utils::{
        bot_builder::BotBuilder, contact::file::AbsoluteFileFolderTrait,
        other::enums::MiraiProtocol,
    },
};
