# The art library

The pictures every app draws from (`candle/docs/yenbot_app_host.md` §15),
so an app looks finished the day it is made and every app looks like part of
one site. Browse them at `/_library`; search them with
`GET /api/v1/library` (`api/library.md`) or `app.library.find(…)`.

## Naming a picture

| Where | How |
|---|---|
| An app's tile (`app.yaml`) | `tile: library/tiles/bell-coral` |
| An empty state | `<yen-empty art="no-recipes" heading="No recipes yet">` |
| Anywhere else | its address, `/_library/<kind>/<id>-<width>.webp` at a size its kind has |
| From code | `await app.library.find({ kind: "illustrations", tag: "cooking" })` |

The check refuses a name the catalogue does not hold (`check.md`,
`references`). A picture an app needs that is not here is drawn to the house
style below and added **to the library**, not to the app, so the next app
has it too.

## The kinds

| Kind | What | Drawn at | Served at |
|---|---|---|---|
| `tiles` | App icons: one subject, full bleed on a flat palette colour; the page rounds the corners | 1024×1024 | 512, 256, 128 |
| `illustrations` | Scenes for headers and onboarding | 1024×576 | 1024×576, 512×288 |
| `empty` | "Nothing here yet": one small subject on cream, lots of room | 768×576 | 600×450, 300×225 |
| `backgrounds` | Soft textures and patterns behind a page or a card | 1024×1024 | 1024, 512 |
| `stickers` | Small cut-out objects and badges, on transparency | 1024×1024, background removed | 256, 128 |

The image model draws at most 1024 a side, so a picture is committed as it
was drawn and the site sizes it down (lossless WebP, transparency kept); a
picture of another shape is cut to its kind's shape from the centre.

## The house style

Every picture is a **soft flat vector illustration**: simple rounded shapes,
smooth edges, no outlines, gentle two-tone shading, a soft long shadow to the
lower right, even light from the upper left, a limited warm palette, and
calm empty space. What never appears: text or lettering of any kind, logos,
photographs, real people's likenesses. People, when a scene has them, are
simple faceless adult figures.

The palette is the design tokens' (`design.md`), by name: **coral, rose,
amber, sage green, teal, sky blue, violet, slate**, with **cream** for grounds.
A tile's field is one of the eight; the subject on it is in cream and the
neighbouring colours.

### The prompt

The image model (Z-Image) reads plain sentences, has no negative prompt, and
varies little with the seed: variety comes from the prompt. Every prompt is
three parts, in order:

1. **The opening, by kind** — the frame:

   | Kind | Opening |
   |---|---|
   | `tiles` | A flat vector illustration. The whole picture, edge to edge, is one plain flat field of soft *‹colour›* with no border and no frame. In the centre sits *‹subject›*. |
   | `illustrations` | A flat vector illustration of *‹scene›*, wide. |
   | `empty` | A flat vector illustration. On a plain soft cream background, in the centre, sits *‹subject›*, small, with wide empty cream space all around it. |
   | `backgrounds` | A flat vector illustration of a soft, quiet pattern filling the whole picture edge to edge: *‹pattern›*. Low contrast, pale tones, nothing in the centre to look at. |
   | `stickers` | A flat vector illustration of a single *‹object›* as a sticker with a thick white border around its outline, centred on a plain flat light grey background. |

2. **The subject**, described concretely: what it is, its colours from the
   palette, where each part sits.

3. **The closing**, the same for every picture:

   > Drawn with simple rounded shapes, smooth edges and no outlines, with
   > gentle two-tone shading on each object and soft long shadows falling to
   > the lower right. Even, diffuse light from the upper left. A clean,
   > modern, friendly flat vector style with a limited warm palette of coral,
   > rose, amber, sage green, teal, sky blue and cream.

Draw at the size the table gives, with `steps: 12` and `format: png`. A
sticker is drawn, then `image_edit {mode: "remove_background"}`.

Example — `tiles/bell-coral`:

> A flat vector illustration. The whole picture, edge to edge, is one plain
> flat field of soft coral orange with no border and no frame. In the centre
> sits a single golden-yellow bell with a small round clapper. The bell has
> gentle two-tone shading: lighter yellow on its upper left, a slightly
> deeper amber on its lower right, and a small cream highlight curving along
> its upper left side. A soft long shadow in a darker coral falls from the
> bell down to the lower right. The bell takes up about half of the picture,
> centred, seen straight on, with plenty of plain coral around it. Drawn with
> simple rounded shapes, …

## Adding a picture

1. Draw it to the house style; draw two or three of a family (vary the
   subject's description, not the seed) and keep the best. Look at it: no
   text, no outline, the right frame for its kind.
2. Commit the PNG as it was drawn at `library/<kind>/<id>.png` — a Zen task
   does this from the link the draw answered with, since a picture is binary.
   An id is lowercase words joined by `-`; a tile's ends in its colour
   (`bell-coral`).
3. Add its entry to `library/catalogue.yaml`, in the same commit:

   ```yaml
   tiles/bell-coral:
     tags: [bell, reminders, alarm]
     seed: 5312857441547061
     prompt: >-
       A flat vector illustration. The whole picture, edge to edge, …
   ```

   `tags` are lowercase words a search would use; `prompt` and `seed` are
   exactly what drew it, so it can be drawn again in kind. A background that
   repeats edge to edge says `tiling: true`.

The gate refuses a line whose picture is not there, a picture with no line,
and anything else under `library/` (`check.md`, `library`). On load, the site
sizes each picture and reads its colour once, caching the result by its
content.
