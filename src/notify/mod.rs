//! Notifications (`candle/docs/yenbot_app_host.md` §11): Web Push to each
//! device an account allowed them on, and the inbox that keeps every one.
//!
//! | Module | |
//! |---|---|
//! | [`vapid`] | the site's push key, and the token each push is sent with |
//! | [`encrypt`] | a message encrypted to one device (RFC 8291) |
//! | [`push`] | the sender thread, posting to each device's push service |
//! | [`devices`] | each account's devices, and the apps it muted |
//! | [`inbox`] | each account's notifications, read and unread |
//! | [`deliver`] | a notification to an account, a group or the admins |

pub mod deliver;
pub mod devices;
pub mod encrypt;
pub mod inbox;
pub mod push;
pub mod vapid;
