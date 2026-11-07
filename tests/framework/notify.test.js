import { Api } from "/_/framework/api.js";
import { Inbox } from "/_/framework/inbox.js";
import { bytesOf, isApple, Notify, stateOf } from "/_/framework/notify.js";
import { fakeFetch } from "./fakes.js";
import { eq, test } from "./harness.js";

test("a device's state says what the bell offers", () => {
  const at = (o) => stateOf({ apple: false, standalone: false, push: true, permission: "default", subscribed: false, ...o });
  eq(at({}), "off");
  eq(at({ apple: true }), "install", "an iPhone in Safari: the home screen first");
  eq(at({ apple: true, standalone: true }), "off");
  eq(at({ push: false }), "unsupported");
  eq(at({ permission: "denied" }), "blocked");
  eq(at({ permission: "granted" }), "off", "allowed, but this device is not subscribed");
  eq(at({ permission: "granted", subscribed: true }), "on");
});

test("an iPad says it is a Mac with a touch screen", () => {
  eq(isApple("Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X)", "iPhone", 5), true);
  eq(isApple("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)", "MacIntel", 5), true);
  eq(isApple("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)", "MacIntel", 0), false);
  eq(isApple("Mozilla/5.0 (Linux; Android 14; Pixel 8)", "Linux armv81", 5), false);
});

test("the push key is read from base64url to its bytes", () => {
  eq(Array.from(bytesOf("AAEC_w")), [0, 1, 2, 255]);
  eq(Array.from(bytesOf("SGk")), [72, 105]);
  eq(bytesOf("BLc4xRzKlKORKWlbdgFaBrrPK3ydWAHo4M0gs0i1oEKgPpWC5cW8OCzVrOQRv-1npXRWk8udnW3oYhIO4475rds").length, 65);
});

/** A device whose subscription the test holds. */
function device({ permission = "default", answer = "granted", apple = false, standalone = false } = {}) {
  const d = { held: null, asked: 0 };
  const sub = (endpoint) => ({
    endpoint,
    toJSON: () => ({ endpoint, keys: { p256dh: "k", auth: "a" } }),
    unsubscribe: async () => {
      d.held = null;
      return true;
    },
  });
  Object.assign(d, {
    apple,
    standalone,
    push: true,
    permission: () => permission,
    ask: async () => {
      d.asked++;
      permission = answer;
      return answer;
    },
    registration: async () => ({
      pushManager: {
        getSubscription: async () => d.held,
        subscribe: async ({ applicationServerKey }) => {
          d.key = applicationServerKey;
          d.held = sub("https://push.example/1");
          return d.held;
        },
      },
    }),
  });
  return d;
}

test("turning notifications on asks, subscribes with the site's key, and tells the site", async () => {
  const f = fakeFetch([
    [200, { key: "AAEC_w" }],
    [200, { subscribed: true }],
    [200, { subscribed: true }],
    [200, { unsubscribed: true }],
  ]);
  const d = device();
  const n = new Notify(new Api("home", f.fetch), d);
  eq(await n.check(), "off");
  eq(await n.enable(), "on");
  eq(n.state.value, "on");
  eq(Array.from(d.key), [0, 1, 2, 255]);
  eq(
    f.calls.map((c) => [c.method, c.url, c.body]),
    [
      ["GET", "/api/v1/push/key", undefined],
      ["POST", "/api/v1/push/subscribe", { endpoint: "https://push.example/1", keys: { p256dh: "k", auth: "a" } }],
    ],
  );
  eq(await n.check(), "on");
  eq([f.calls[2].method, f.calls[2].url], ["POST", "/api/v1/push/subscribe"], "told again, for whoever is signed in");
  eq(await n.disable(), "off");
  eq(f.calls[3].body, { endpoint: "https://push.example/1" });
});

test("a person who says no, or an iPhone not yet on the home screen, is not subscribed", async () => {
  const f = fakeFetch([]);
  const no = new Notify(new Api("home", f.fetch), device({ answer: "denied" }));
  eq(await no.enable(), "blocked");
  const phone = device({ apple: true });
  const iphone = new Notify(new Api("home", f.fetch), phone);
  eq(await iphone.enable(), "install");
  eq(phone.asked, 0, "not asked: Safari would refuse");
  eq(f.calls.length, 0);
});

test("the inbox is read, and a notification marked read at once", async () => {
  const items = [
    { id: "n2", app: "chores", title: "Bo finished the bins", body: "", created_ms: 2, read_ms: null },
    { id: "n1", app: "chores", title: "Mai finished the dishes", body: "", created_ms: 1, read_ms: null },
  ];
  const f = fakeFetch([
    [200, { items, unread: 2, muted: ["meals"] }],
    [200, { read: 1 }],
    [200, { read: 1 }],
  ]);
  const inbox = new Inbox(new Api("home", f.fetch));
  await inbox.load();
  eq([inbox.unread.value, inbox.muted.value], [2, ["meals"]]);
  const reading = inbox.read("n1");
  eq(inbox.unread.value, 1, "at once");
  await reading;
  await inbox.read("n1");
  eq(f.calls.length, 2, "read once");
  await inbox.readAll();
  eq(inbox.unread.value, 0);
  eq(
    f.calls.map((c) => `${c.method} ${c.url}`),
    ["GET /api/v1/inbox", "POST /api/v1/inbox/n1/read", "POST /api/v1/inbox/all/read"],
  );
});
