//! Integrations (`candle/docs/yenbot_app_host.md` §19): an app's keys for
//! the services it talks to ([`secrets`]), its workflows' calls out to the
//! hosts its manifest lists ([`outbound`]), and the calls other services
//! make in to its hooks ([`hooks`]).

pub mod hooks;
pub mod outbound;
pub mod secrets;

#[cfg(test)]
pub mod fake;
