use std::sync::Mutex;

use super::*;
use crate::ai::fake::FakeZen;

fn ask(messages: &[(&str, &str)]) -> Ask {
    Ask {
        system: Some("You help plan meals.".into()),
        messages: messages
            .iter()
            .map(|(r, c)| Said {
                role: r.to_string(),
                content: c.to_string(),
            })
            .collect(),
        ..Ask::default()
    }
}

fn chunk(piece: &str) -> String {
    format!(
        "data: {}",
        json!({"choices": [{"delta": {"content": piece}}]})
    )
}

const USAGE: &str = "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":40,\"completion_tokens\":6,\"total_tokens\":46}}";

/// **The app's own system prompt and messages go to the model as given;
/// its answer arrives a piece at a time, Zen's own events left out.**
#[test]
fn an_answer_arrives_as_it_is_written() {
    let zen = FakeZen::chats(vec![vec![
        "event: status".into(),
        "data: {\"phase\":\"prefill\"}".into(),
        String::new(),
        chunk("Phở "),
        chunk("on Sunday."),
        USAGE.into(),
        "data: [DONE]".into(),
    ]]);
    let pieces = Mutex::new(Vec::new());
    let a = chat(&zen, &ask(&[("user", "What's for dinner?")]), &mut |p| {
        pieces.lock().unwrap().push(p.to_string())
    })
    .unwrap()
    .unwrap();
    assert_eq!(a.content, "Phở on Sunday.");
    assert_eq!((a.tokens_in, a.tokens_out), (Some(40), Some(6)));
    assert_eq!(*pieces.lock().unwrap(), ["Phở ", "on Sunday."]);
    let sent = &zen.sent()[0];
    assert_eq!(sent["model"], "passthrough");
    assert_eq!(
        sent["messages"],
        json!([
            {"role": "system", "content": "You help plan meals."},
            {"role": "user", "content": "What's for dinner?"}
        ])
    );
}

/// **An answer held to a schema is read whole and checked; one that does
/// not fit is asked for again, told why.**
#[test]
fn an_answer_held_to_a_schema_is_asked_again_once() {
    let zen = FakeZen::chats(vec![
        vec![chunk("Sure! Here: {\"dish\": 3}")],
        vec![chunk("```json\n{\"dish\": \"Phở\"}\n```")],
    ]);
    let mut a = ask(&[("user", "One dish")]);
    a.schema = Some(
        json!({"type": "object", "required": ["dish"], "properties": {"dish": {"type": "string"}}}),
    );
    let answer = chat(&zen, &a, &mut |_| panic!("not streamed"))
        .unwrap()
        .unwrap();
    assert_eq!(answer.json, Some(json!({"dish": "Phở"})));
    let second = &zen.sent()[1]["messages"];
    let told = second.as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        told.starts_with("That does not match the schema: it is not JSON"),
        "{told}"
    );
    assert!(zen.sent()[0]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("Answer with JSON alone"));
}

#[test]
fn what_cannot_be_asked_is_said() {
    let zen = FakeZen::chats(vec![vec![
        "event: turn_error".into(),
        "data: {\"message\":\"the model is busy\"}".into(),
    ]]);
    assert_eq!(
        chat(&zen, &ask(&[]), &mut |_| {}).unwrap(),
        Err("a chat has at least one message".into())
    );
    assert_eq!(
        chat(&zen, &ask(&[("assistant", "hi")]), &mut |_| {}).unwrap(),
        Err("the last message is the person's, `role: \"user\"`".into())
    );
    assert_eq!(
        chat(&zen, &ask(&[("system", "x"), ("user", "hi")]), &mut |_| {}).unwrap().unwrap_err(),
        "`system` is not a message's role: `user` or `assistant`; the app's own instructions are `system`"
    );
    assert_eq!(
        chat(&zen, &ask(&[("user", "hi")]), &mut |_| {}).unwrap(),
        Err("the model is busy".into())
    );
}
