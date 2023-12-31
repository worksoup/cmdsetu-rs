pub(crate) use mirai_j4rs::{
    contact::{
        bot::BotBuilder,
        contact_trait::{ContactOrBotTrait, ContactTrait, SendMessageSupportedTrait},
        group::Group,
        Member,
    },
    event::{event_trait::MessageEventTrait, message::GroupMessageEvent},
    message::{message_trait::MessageTrait, At, MessageChain, PlainText},
    utils::other::enums::MiraiProtocol,
};
