import { describe, Reporter } from "/_/framework/report.js";
import { eq, test } from "./harness.js";

test("an error is posted once; its repeats are counted, then posted on flush", () => {
  const posted = [];
  const r = new Reporter("recipes", (p) => posted.push(p));
  eq(r.error("TypeError: x", { stack: "at a.js:1", page: "/a/recipes/" }), true);
  eq(r.error("TypeError: x"), false);
  eq(r.error("TypeError: x"), false);
  eq(r.flush(), 1);
  eq(r.flush(), 0, "nothing new to count");
  eq(posted, [
    { app: "recipes", page: "/a/recipes/", message: "TypeError: x", stack: "at a.js:1" },
    { app: "recipes", page: "/a/recipes/", message: "TypeError: x", stack: "at a.js:1", repeats: 2 },
  ]);
});

test("a page that goes wrong in a loop posts at most its limit", () => {
  const posted = [];
  const r = new Reporter("recipes", (p) => posted.push(p), { limit: 2 });
  for (let i = 0; i < 5; i++) r.error(`error ${i}`);
  eq(
    posted.map((p) => p.message),
    ["error 0", "error 1"],
  );
  r.error("error 4");
  eq(r.flush(), 0, "an error never posted has no count to post");
});

test("whatever was thrown is described", () => {
  const e = new TypeError("r.title is undefined");
  eq(describe(e).message, "TypeError: r.title is undefined");
  eq(describe("plain words"), { message: "plain words", stack: "" });
  eq(describe({ code: 7 }), { message: '{"code":7}', stack: "" });
  eq(describe(undefined), { message: "undefined", stack: "" });
});
