import { moved, size, usageByApp } from "/_/site/admin-summary.js";
import { eq, test } from "./harness.js";

test("each app's use is summed over the period, most opened first", () => {
  const byDay = [
    { app: "recipes", key: "2026-11-06", opens: 3, accounts: 2, chats: 1, pictures: 0, questions: 0 },
    { app: "recipes", key: "2026-11-07", opens: 1, accounts: 1, chats: 0, pictures: 2, questions: 1 },
    { app: "chores", key: "2026-11-07", opens: 9, accounts: 3, chats: 0, pictures: 0, questions: 0 },
    { app: "chores", key: "2026-11-05", opens: 0, accounts: 0, chats: 4, pictures: 0, questions: 0 },
  ];
  const byAccount = [
    { app: "recipes", key: "1", opens: 3 },
    { app: "recipes", key: "2", opens: 1 },
    { app: "recipes", key: "", opens: 0 },
    { app: "chores", key: "1", opens: 9 },
  ];
  eq(usageByApp(byDay, byAccount), [
    { app: "chores", opens: 9, accounts: 1, chats: 4, pictures: 0, questions: 0, days: 1 },
    { app: "recipes", opens: 4, accounts: 2, chats: 1, pictures: 2, questions: 1, days: 2 },
  ]);
});

test("an app moves one place, and not past either end", () => {
  eq(moved(["a", "b", "c"], "b", -1), ["b", "a", "c"]);
  eq(moved(["a", "b", "c"], "b", 1), ["a", "c", "b"]);
  eq(moved(["a", "b", "c"], "a", -1), ["a", "b", "c"]);
  eq(moved(["a", "b", "c"], "c", 1), ["a", "b", "c"]);
  eq(moved(["a"], "x", 1), ["a"]);
});

test("a size is said as people read it", () => {
  eq([size(512), size(20_480), size(18_234_011)], ["512 B", "20 KB", "17.4 MB"]);
});
