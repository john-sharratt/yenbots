// Whether a chosen file is one a picker takes: its type against the
// picker's `accept` list (`image/*`, `image/png`, `.pdf`) and its size
// against a ceiling. The site checks the bytes again on upload (§12.6);
// this tells the person at once, beside the picker.

/** Whether a file named `name` of `type` is one `accept` takes; every file when it is empty. */
export function accepted(accept, { name = "", type = "" }) {
  const wanted = String(accept ?? "")
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  if (wanted.length === 0) return true;
  const t = type.toLowerCase();
  const n = name.toLowerCase();
  return wanted.some((w) =>
    w.startsWith(".") ? n.endsWith(w) : w.endsWith("/*") ? t.startsWith(w.slice(0, -1)) : t === w,
  );
}

/** A size in words: "820 KB", "3.4 MB". */
export function sizeOf(bytes) {
  if (bytes < 1000) return `${bytes} bytes`;
  if (bytes < 1_000_000) return `${Math.round(bytes / 1000)} KB`;
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

/** Why a file cannot be taken, or null when it can. */
export function refusal(file, { accept, maxBytes }) {
  if (!accepted(accept, file)) return "isn't a kind of file this takes";
  if (maxBytes && file.size > maxBytes) return `is ${sizeOf(file.size)}; the most is ${sizeOf(maxBytes)}`;
  return null;
}
