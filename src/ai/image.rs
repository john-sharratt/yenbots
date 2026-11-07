//! A picture drawn by Zen's image engine (§14): described, or — to change
//! one the app holds — described with that picture as its reference, which
//! the engine keeps the drawing close to. The engine tells its progress a
//! line at a time and ends with the picture, a PNG; the caller keeps it as
//! a file of the app.

use anyhow::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};

use super::zen::Zen;

/// The longest side a picture may have: what the image model draws (§15.1).
pub const MAX_SIDE: u32 = 1024;

/// How closely an edit keeps to its reference when the app does not say.
pub const HOLD: f32 = 0.5;

/// What an app asks to be drawn.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Draw {
    pub prompt: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub seed: Option<u64>,
    /// One of the app's pictures, by file id, to change rather than draw anew.
    #[serde(default)]
    pub from: Option<String>,
    /// How closely the change keeps to it, 0 to 0.95.
    #[serde(default)]
    pub hold: Option<f32>,
}

/// A picture drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawn {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub seed: Option<u64>,
}

impl Draw {
    /// The size it is drawn at — a square of the longest side when not said —
    /// or why that size cannot be drawn.
    pub fn size(&self) -> std::result::Result<(u32, u32), String> {
        let (w, h) = (
            self.width.unwrap_or(MAX_SIDE),
            self.height.unwrap_or(MAX_SIDE),
        );
        for side in [w, h] {
            if !(64..=MAX_SIDE).contains(&side) || side % 16 != 0 {
                return Err(format!(
                    "a picture's sides are from 64 to {MAX_SIDE}, each a multiple of 16; {side} is not"
                ));
            }
        }
        Ok((w, h))
    }

    /// The request the engine is sent; `reference` is the picture changed.
    pub fn body(&self, reference: Option<&[u8]>) -> std::result::Result<Value, String> {
        let prompt = self.prompt.trim();
        if prompt.is_empty() {
            return Err("a picture is described: its `prompt`".into());
        }
        let (width, height) = self.size()?;
        let mut body = json!({"prompt": prompt, "width": width, "height": height});
        if let Some(seed) = self.seed {
            body["seed"] = json!(seed);
        }
        if let Some(bytes) = reference {
            body["reference"] = json!(STANDARD.encode(bytes));
            body["reference_hold"] = json!(self.hold.unwrap_or(HOLD).clamp(0.0, 0.95));
        }
        Ok(body)
    }
}

/// Draw `draw`, `reference` its picture to change, handing each line of
/// progress — `{"event": "step", "done", "total"}`, `waiting`, `loading` —
/// to `progress`. `Err` is what went wrong, said for the app.
pub fn draw(
    zen: &dyn Zen,
    draw: &Draw,
    reference: Option<&[u8]>,
    progress: &mut dyn FnMut(&Value),
) -> Result<std::result::Result<Drawn, String>> {
    let body = match draw.body(reference) {
        Ok(b) => b,
        Err(why) => return Ok(Err(why)),
    };
    let mut done: Option<std::result::Result<Drawn, String>> = None;
    zen.image(&body, &mut |line: &str| {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match v["event"].as_str() {
            Some("done") => {
                let png = v["png_base64"]
                    .as_str()
                    .and_then(|b| STANDARD.decode(b).ok());
                done = Some(match png {
                    Some(png) => Ok(Drawn {
                        png,
                        width: v["width"].as_u64().unwrap_or(0) as u32,
                        height: v["height"].as_u64().unwrap_or(0) as u32,
                        seed: v["seed"].as_u64(),
                    }),
                    None => Err("the image engine's picture could not be read".into()),
                });
            }
            Some("error") => {
                let detail = v["detail"]
                    .as_str()
                    .or(v["error"].as_str())
                    .unwrap_or("it failed");
                done = Some(Err(format!("the picture was not drawn: {detail}")));
            }
            Some(_) => progress(&v),
            None => {}
        }
    })?;
    Ok(done.unwrap_or_else(|| Err("the image engine stopped before the picture was drawn".into())))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ai::fake::FakeZen;

    fn ask(prompt: &str) -> Draw {
        Draw {
            prompt: prompt.into(),
            ..Draw::default()
        }
    }

    /// **A picture is drawn at the size asked, its progress told, the PNG
    /// it ends with kept; a change sends its picture as the reference.**
    #[test]
    fn a_picture_is_drawn_and_its_progress_told() {
        let png = b"\x89PNG\r\n\x1a\nnot really";
        let zen = FakeZen::images(vec![vec![
            "{\"event\":\"loading\"}".into(),
            "{\"event\":\"step\",\"done\":1,\"total\":24,\"what\":\"draw\"}".into(),
            format!("{{\"event\":\"done\",\"width\":512,\"height\":256,\"png_base64\":\"{}\",\"seed\":7}}", STANDARD.encode(png)),
        ]]);
        let seen = Mutex::new(Vec::new());
        let mut d = ask("A teapot, soft flat vector");
        d.width = Some(512);
        d.height = Some(256);
        d.from = Some("f1".into());
        let drawn = draw(&zen, &d, Some(b"old picture"), &mut |v| {
            seen.lock()
                .unwrap()
                .push(v["event"].as_str().unwrap().to_string())
        })
        .unwrap()
        .unwrap();
        assert_eq!(
            drawn,
            Drawn {
                png: png.to_vec(),
                width: 512,
                height: 256,
                seed: Some(7)
            }
        );
        assert_eq!(*seen.lock().unwrap(), ["loading", "step"]);
        assert_eq!(
            zen.sent()[0],
            json!({"prompt": "A teapot, soft flat vector", "width": 512, "height": 256,
                   "reference": STANDARD.encode(b"old picture"), "reference_hold": 0.5})
        );
    }

    #[test]
    fn what_cannot_be_drawn_is_said() {
        let zen = FakeZen::images(vec![vec![
            "{\"event\":\"error\",\"error\":\"prompt_declined\",\"detail\":\"that is not something it draws\"}".into(),
        ]]);
        assert_eq!(
            draw(&zen, &ask(" "), None, &mut |_| {}).unwrap(),
            Err("a picture is described: its `prompt`".into())
        );
        let mut big = ask("x");
        big.width = Some(2048);
        assert_eq!(
            draw(&zen, &big, None, &mut |_| {}).unwrap(),
            Err("a picture's sides are from 64 to 1024, each a multiple of 16; 2048 is not".into())
        );
        assert_eq!(
            draw(&zen, &ask("x"), None, &mut |_| {}).unwrap(),
            Err("the picture was not drawn: that is not something it draws".into())
        );
    }
}
