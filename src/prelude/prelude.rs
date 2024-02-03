pub(crate) use mirai_j4rs::{
    auth::bot_authorization::BotAuthorization,
    contact::{
        BotBuilder, ContactOrBotTrait, ContactTrait, Group, Member, SendMessageSupportedTrait,
    },
    event::{event_trait::MessageEventTrait, message::GroupMessageEvent},
    message::{
        data::{At, MessageChain, PlainText},
        MessageTrait,
    },
    utils::other::enums::MiraiProtocol,
};
