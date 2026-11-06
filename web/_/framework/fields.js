// A collection's fields as a form's controls (§8.6): which control each
// type is, what it is labelled, and its value both ways — the record's value
// shown in the control, and what was typed turned back into the record's
// value. The site checks every write (§7.5); a form only shapes it.

/** The control a field is edited with, or null for one a form does not build. */
export function controlOf(field) {
  switch (field.type) {
    case "text":
      return field.long ? "textarea" : "text";
    case "number":
      return "number";
    case "bool":
      return "checkbox";
    case "time":
      return "datetime-local";
    case "date":
      return "date";
    case "list":
      return field.of === "text" ? "tags" : null;
    default:
      // A file, an object or a reference is a page's own to build.
      return null;
  }
}

/** A field's name as a label: `due_ms` is "Due", `shopping-list` is "Shopping list". */
export function labelOf(name) {
  const words = name.replace(/_ms$/, "").split(/[-_]+/).filter(Boolean).join(" ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** A time zone at a fixed offset from UTC, in minutes east: Sydney's summer is 660. */
export function fixedZone(minutesEast) {
  const shift = minutesEast * 60_000;
  return {
    toLocal: (ms) => new Date(ms + shift).toISOString().slice(0, 16),
    fromLocal: (text) => Date.parse(`${text}:00Z`) - shift,
  };
}

/** The value `field` holds, as its control shows it. */
export function toInput(field, value, zone) {
  const control = controlOf(field);
  if (control === "checkbox") return value === true;
  if (value === undefined || value === null) return "";
  if (control === "datetime-local") return zone.toLocal(value);
  if (control === "tags") return value.join(", ");
  return String(value);
}

/** Why what was typed is not a value of its field. */
export class FieldError extends Error {
  constructor(field, reason) {
    super(reason);
    this.name = "FieldError";
    this.field = field;
  }
}

/**
 * What was typed into `name`'s control as the record's value: null for
 * nothing typed, which a new record leaves out and a change clears.
 */
export function fromInput(name, field, raw, zone) {
  const control = controlOf(field);
  if (control === "checkbox") return raw === true;
  const text = typeof raw === "string" ? raw : "";
  if (text.trim() === "") return null;
  switch (control) {
    case "number": {
      const n = Number(text);
      if (!Number.isFinite(n)) throw new FieldError(name, "must be a number");
      return n;
    }
    case "datetime-local": {
      const ms = zone.fromLocal(text);
      if (!Number.isFinite(ms)) throw new FieldError(name, "must be a date and time");
      return ms;
    }
    case "tags":
      return text
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);
    default:
      return text;
  }
}

/**
 * A form's values as a write: for a new record, what was typed; for a
 * change, only what differs from `record`, a cleared field as null.
 */
export function writeOf(fields, values, record) {
  const out = {};
  for (const [name, value] of Object.entries(values)) {
    if (!(name in fields)) continue;
    if (!record) {
      if (value !== null) out[name] = value;
      continue;
    }
    const held = record[name] ?? null;
    if (JSON.stringify(held) !== JSON.stringify(value)) out[name] = value;
  }
  return out;
}
