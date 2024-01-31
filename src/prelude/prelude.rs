pub(crate) use mirai_j4rs::{
    auth::bot_authorization::BotAuthorization,
    contact::{
        bot::BotBuilder,
        contact_trait::{ContactOrBotTrait, ContactTrait, SendMessageSupportedTrait},
        group::Group,
        Member,
    },
    event::{event_trait::MessageEventTrait, message::GroupMessageEvent},
    message::{
        data::{at::At, message_chain::MessageChain, plain_text::PlainText},
        message_trait::MessageTrait,
    },
    utils::other::enums::MiraiProtocol,
};
