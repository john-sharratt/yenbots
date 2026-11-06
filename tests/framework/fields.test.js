import { controlOf, FieldError, fixedZone, fromInput, labelOf, toInput, writeOf } from "/_/framework/fields.js";
import { eq, test } from "./harness.js";

const sydney = fixedZone(660);

test("each field type has its control", () => {
  eq(
    [
      { type: "text" },
      { type: "text", long: true },
      { type: "number" },
      { type: "bool" },
      { type: "time" },
      { type: "date" },
      { type: "list", of: "text" },
      { type: "list", of: "number" },
      { type: "file" },
      { type: "ref", coll: "cooks" },
    ].map(controlOf),
    ["text", "textarea", "number", "checkbox", "datetime-local", "date", "tags", null, null, null],
  );
});

test("a field's name is its label", () => {
  eq(["title", "due_ms", "shopping-list", "best_by"].map(labelOf), ["Title", "Due", "Shopping list", "Best by"]);
});

test("a value is shown in its control and read back", () => {
  const due = { type: "time" };
  // 7:30 on 8 November 2026 in Sydney is 20:30 on the 7th in UTC.
  const ms = Date.UTC(2026, 10, 7, 20, 30);
  eq(toInput(due, ms, sydney), "2026-11-08T07:30");
  eq(fromInput("due_ms", due, "2026-11-08T07:30", sydney), ms);
  eq(toInput({ type: "list", of: "text" }, ["phở", "bún"], sydney), "phở, bún");
  eq(fromInput("tags", { type: "list", of: "text" }, " phở, ,bún ", sydney), ["phở", "bún"]);
  eq(fromInput("serves", { type: "number" }, "4", sydney), 4);
  eq(fromInput("done", { type: "bool" }, true, sydney), true);
  eq(toInput({ type: "bool" }, undefined, sydney), false);
  eq(toInput({ type: "text" }, undefined, sydney), "");
  eq(fromInput("title", { type: "text" }, "  ", sydney), null, "nothing typed");
});

test("what is not a value of its field says so", () => {
  let thrown = null;
  try {
    fromInput("serves", { type: "number" }, "four", sydney);
  } catch (e) {
    thrown = e;
  }
  eq([thrown instanceof FieldError, thrown.field, thrown.message], [true, "serves", "must be a number"]);
});

test("a new record is what was typed; a change, only what differs", () => {
  const fields = { title: { type: "text" }, serves: { type: "number" }, notes: { type: "text" } };
  const typed = { title: "Phở", serves: 4, notes: null, stray: 1 };
  eq(writeOf(fields, typed, null), { title: "Phở", serves: 4 });
  const held = { id: "r1", title: "Phở", serves: 2, notes: "hot" };
  eq(writeOf(fields, typed, held), { serves: 4, notes: null });
});
