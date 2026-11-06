# The design system

How an app looks: the tokens every page is drawn in, the few an app may
change for itself, and every component with what it takes and what it
raises. The design is `candle/docs/yenbot_app_host.md` §8.5 and §8.7.

**Open `/_design` first.** It shows every token and every component in every
state, light and dark side by side, at a phone's width or a desktop's;
`/_design?app=<id>` shows it all in that app's theme, and `?part=buttons`
just one section. `/_design/vietnamese` is the type's test page.

## The rules, each with its reason

- **Tokens, never literal colours or sizes.** Write `var(--yen-accent)`,
  `var(--yen-space-4)`. Then light and dark, a theme and a change to the
  platform's look reach the app without an edit.
- **A component before a hand-built element.** `<yen-button>`, not a styled
  `<button>`: the component has its states, its focus, its screen-reader
  words and its motion already.
- **Hide what the caller may not do**, rather than show it disabled.
- **Text cut short is cut sideways** (`overflow-x: clip; text-overflow:
  ellipsis`), never with `overflow: hidden` on a line: a Vietnamese capital
  carries marks above the line box, and a vertical clip cuts them off.

## What every app page has

The site puts `/_/yen.css` and the app's theme (`/_/theme/<id>.css`) at the
top of every app page's head, so `index.html` names neither. `yen.css`
carries the tokens, the two faces and the page's own type; the theme follows
it.

## The tokens

`web/_/ui/tokens.css`, by what each is for:

| Kind | Names |
|---|---|
| Surface | `--yen-bg` (the page), `--yen-surface` (a card), `--yen-surface-2`, `-3` (sunken fills), `--yen-line`, `--yen-line-strong` (an input's edge) |
| Ink | `--yen-ink`, `--yen-ink-2` (secondary, still readable), `--yen-ink-3` (only what cannot be used) |
| Accent | `--yen-accent`, `--yen-on-accent` (ink on it), `--yen-accent-soft` (a tinted fill), `--yen-focus` |
| State | `--yen-danger`, `--yen-on-danger`, `--yen-success`, `--yen-warning`, each with `-soft`; `--yen-scrim` behind a dialog |
| Over a photo | `--yen-overlay`, `--yen-overlay-strong`, `--yen-on-overlay` |
| Palette | `--yen-coral`, `rose`, `amber`, `sage`, `teal`, `sky`, `violet`, `slate`; `--yen-chart-1` to `-6` |
| Type | `--yen-font-text` (Be Vietnam Pro), `--yen-font-display` (Fraunces), `--yen-font-heading`; `--yen-text-xs` to `-3xl`; `--yen-weight-regular` to `-bold`; `--yen-leading`, `--yen-leading-tight`, `--yen-measure` |
| Space | `--yen-space-1` (0.25rem) to `-8` (4rem); `--yen-control-height`; `--yen-tap` (the smallest thing a finger presses) |
| Corners | `--yen-radius-1`, `-2`, `-3`, `--yen-radius-button`, `--yen-radius-pill` |
| Lift | `--yen-shadow-1` (a card), `-2` (raised), `-3` (a sheet or dialog) |
| Motion | `--yen-duration-1` to `-3`, `--yen-ease-out`, `-in-out`, `-spring`; all still when the device asks for less motion |

Every one has a light and a dark value; a page follows the device.

## An app's theme

An app may change a few named tokens for itself, in its manifest:

```yaml
theme:
  accent: "#b03a5b"        # the app's colour: buttons, links, its tile
  accent-dark: "#f07a95"   # the same in dark mode; lightened from accent when absent
  surface: warm            # warm | plain | cool | rose | sage
  radius: round            # soft | round | square
  type: display            # text | display — the face for headings
  density: roomy           # compact | normal | roomy
```

Only these names exist. **The accent must be readable**: at least 4.5:1
against the page and a card of its surface, in light mode and in dark. A
manifest whose accent is not is refused, saying by how much and which way to
go — "the accent #f9a8c0 is 1.73:1 against the warm surface in light mode,
and text needs 4.5:1 to be read — choose a deeper colour". The ink on a
button is chosen for the accent: white, or the dark page's colour, whichever
reads better.

## Icons

`<yen-icon name="plus">`, in the ink around it at the size of the text
around it; `label` when it means something on its own. The names are in
`web/_/ui/icons.js` and on the catalogue. A picture — a tile, an
illustration — is the art library's, not an icon.

## The components

Every one is defined by `/_/yen.js`. A property set from code is written
`.name=${…}`; an attribute `name="…"`. Events carry their value as `detail`.

### Pages and layout

| Tag | Takes | Raises |
|---|---|---|
| `<yen-page>` | `heading`, `back` (the address the arrow goes to), `wide`; slot `actions` | |
| `<yen-card>` | `href` (the whole card a link), `flat`; slot `media` (a picture, edge to edge) | |
| `<yen-list>` | `.items`, `.item` (a record → its row), `.key` (`id` when absent), `loading`, `empty`, `empty-icon`, `divided`; slots `empty`, `empty-action` | |
| `<yen-list-item>` | `href`, `headline`, `supporting`; slots `start`, `end`, `supporting` | |
| `<yen-grid>` | `size`: small, medium, large — how wide a column is at least | |
| `<yen-tabs>` | `.tabs` (`[{id, label, icon}]`), `selected`, `label`; each panel a child with `slot` its id | `change` (the id) |

### Doing things

| Tag | Takes | Raises |
|---|---|---|
| `<yen-button>` | `variant`: primary, secondary, ghost, danger; `size="small"`, `icon`, `href`, `type` (`submit` submits the form it is in), `loading`, `disabled`, `wide` | `click` |
| `<yen-icon-button>` | `icon`, `label` (required), `variant`, `href`, `pressed` (a toggle) | `click` |
| `<yen-fab>` | `icon`, `href`, `compact`; its words are its content | `click` |
| `<yen-sheet>` | `heading`, `open`; slot `actions` | `close` |
| `<yen-dialog>` | `heading`, `confirm`, `cancel`, `tone="danger"`, `open` | `confirm`, `cancel`, `close` |
| `<yen-toasts>` | — the shell holds one | |

From code: `toast("Saved")`, `toast("Deleted", {action: {label: "Undo", run}})`,
`toast("Couldn't save", {tone: "danger"})`; and `if (await ask({heading:
"Delete this recipe?", confirm: "Delete", tone: "danger"}))`.

### Fields

Each takes `label`, `name`, `.value`, `hint`, `error`, `required` and
`disabled`; raises `input` as it changes and `change` when settled; and takes
part in a `<form>` as a native control does.

| Tag | Also takes | Value |
|---|---|---|
| `<yen-text-field>` | `type` (text, email, number, tel, url, search, password), `placeholder`, `icon`, `inputmode`, `autocomplete` | the text |
| `<yen-text-area>` | `rows`, `placeholder` | the text |
| `<yen-select>` | `.options` (values, or `{value, label}`), `placeholder` | the value chosen |
| `<yen-date-picker>` | `type`: date, time, datetime; `min`, `max` | `YYYY-MM-DD`, `HH:MM`, or milliseconds |
| `<yen-toggle>` | `checked` | on or off |
| `<yen-checkbox>` | `checked`, `indeterminate` | ticked or not |

`<yen-form>` builds a whole form from a collection's fields
(`framework.md`).

### Showing things

| Tag | Takes | Raises |
|---|---|---|
| `<yen-empty>` | `icon`, `heading`; its words; slot `action` | |
| `<yen-skeleton>` | `shape`: text (`lines`), circle, block | |
| `<yen-avatar>` | `name`, `src`, `size`: small, medium, large — initials on the name's own colour when there is no picture | |
| `<yen-tag>` | `tone`: neutral, accent, success, warning, danger; `icon`, `removable` | `remove` |
| `<yen-progress>` | `.value` (0 to 1; absent when not known), `label`, `kind="spinner"` | |
| `<yen-chart>` | `kind`: bar, line; `.rows` (`[{label, value}]`), `label`, `.format`, `empty` | |
| `<yen-calendar>` | `.value`, `.marks` (dates with something on) | `change` (the date), `month` |

### Pictures and AI

| Tag | Takes | Raises |
|---|---|---|
| `<yen-image-picker>` | `label`, `accept`, `max-bytes`, `multiple`, `hint`, `error`; `.files` is what was chosen | `change` (the files) |
| `<yen-photo-grid>` | `.photos` (`[{src, thumb, alt}]`), `size` | `open` (the index) |
| `<yen-chat>` | `.messages` (`[{role, text, pending}]`), `busy`, `placeholder`, `heading` | `send` (the words) |
| `<yen-draw>` | `.pictures` (`[{src, alt}]`), `busy`, `.progress`, `placeholder`, `label`, `error` | `draw` (the words), `pick` (the picture) |

A picker chooses and shows; sending the files is the page's. The chat and
drawing panels show and ask; the page holds the conversation and calls the AI.

## Type and Vietnamese

Two self-hosted faces with every Vietnamese letter: **Be Vietnam Pro** for
text, at four weights, and **Fraunces** for display — headings, unless the
theme says `type: text`. Each is split by range, so a page loads only what
its text uses. Lines are never set tighter than `--yen-leading-tight` (1.4),
so stacked marks clear the line above. Text is stored composed (NFC), lists
sort by `Intl.Collator("vi")`, and search ignores accents.

## Where it lives

| File | |
|---|---|
| `web/_/yen.css` | the page's look, with the tokens and the faces |
| `web/_/ui/tokens.css`, `fonts.css` | the tokens; the faces |
| `web/_/ui/styles.js` | the styles components share: button, control, field |
| `web/_/ui/components/` | one component per file, named for its tag |
| `web/_/ui/*.js` | their arithmetic — months, chart scales, toasts, initials — tested in QuickJS |
| `web/_/icons/sprite.svg` | the icons (Lucide, ISC) |
| `web/_/fonts/` | the faces (SIL Open Font License) |
| `web/_design/` | the catalogue and the Vietnamese test page |
| `src/apps/theme/` | an app's theme: its names, its contrast check, its stylesheet |

A change here is a release of the binary. The tests hold the icon names to
the sprite, every face to the Vietnamese letters, the house palette to AA on
every surface, and every theme to readable.
