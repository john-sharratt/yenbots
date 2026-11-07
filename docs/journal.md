# The journal

Claude does not remember between chats; the `yenbot` journal does
(`candle/docs/yenbot_app_host.md` §17.6). It is a journal of Zen's memory,
read and written from claude.ai and from Zen's workers alike.

## What goes in it

- **Per app**: what it is for, what Yen decided and why, what she asked for
  that is not done yet. One entry per app, named for its id, added to as
  things are decided.
- **Yen's taste**: what she likes and does not — "no gradients", "bigger
  type on the kids' apps", "warm colours" — which every app follows.
- **What was there before** a change she might want back: the words, the
  colour, the layout — so "put it back as it was" can be done by failing
  forward.

## When

- **Before changing an app**: `journal_grep` for its id and read what is
  there, and the taste entries.
- **As each thing is decided**: `journal_note`, with the reason — "made
  the recipes private: Yen wants them just for the family".
- **When she asks for something back**: read what was noted, put it back in
  a new edit.

Only what actually happened goes in: never a note of something not decided,
or a guess at what she meant.
