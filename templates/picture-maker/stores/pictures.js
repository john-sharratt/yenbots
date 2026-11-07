// Drawing: what was just drawn, how far the drawing has got, and the
// pictures kept. A picture drawn is already one of the app's files; keeping
// it is a record that names it.
import { app, computed, signal } from "/_/yen.js";

export const pictures = app.collection("pictures", { order: "-created_ms" });

/** The house look every picture is drawn in, after the person's words. */
const STYLE = ", soft flat vector illustration, gentle shading, warm friendly colours";

/** `[{src, alt, file, prompt}]`: what this visit has drawn, newest first. */
export const drawn = signal([]);
export const busy = signal(false);
/** 0 to 1 while a picture is drawn; null when how far is not known. */
export const progress = signal(null);
export const error = signal("");

/** Draw `words`. */
export async function draw(words) {
  busy.value = true;
  progress.value = null;
  error.value = "";
  try {
    const { file } = await app.ai.image(
      { prompt: `${words}${STYLE}`, width: 768, height: 768 },
      { onProgress: (p) => (progress.value = p.total ? p.done / p.total : null) },
    );
    drawn.value = [{ src: file.url, alt: words, file: file.id, prompt: words }, ...drawn.value];
  } catch (e) {
    error.value = e.detail ?? "The picture couldn't be drawn just now. Try again in a moment.";
  } finally {
    busy.value = false;
  }
}

/** Keep `picture`, one this visit drew. */
export function keep(picture) {
  return pictures.add({ prompt: picture.prompt, file: picture.file });
}

/** The pictures kept, as `<yen-photo-grid>` shows them. */
export const kept = computed(() =>
  pictures.value.map((p) => ({
    src: app.files.url(p.file),
    thumb: app.files.url(p.file, { thumb: true }),
    alt: p.prompt,
  })),
);
