pub(crate) use mirai_j4rs::{
    auth::bot_authorization::BotAuthorization,
    contact::{ContactOrBotTrait, ContactTrait, Group, Member, SendMessageSupportedTrait},
    event::{event_trait::MessageEventTrait, message::GroupMessageEvent},
    message::{
        data::{At, MessageChain, PlainText},
        MessageTrait,
    },
    utils::{bot_builder::BotBuilder, other::enums::MiraiProtocol, EnvConfig},
};
