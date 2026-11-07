// `app.notify` (§11.1): whether this device gets the site's notifications,
// and turning them on — the browser asked, the device subscribed with the
// site's push key, the subscription sent to the site. An iPhone or iPad
// gets them only once the site is on its home screen and opened from
// there, so until then its state is `install`, and the bell walks the
// person through that.

import { signal } from "../vendor/signals-core.mjs";

/**
 * What a device can do, as a state:
 * - `install` — an iPhone or iPad not opened from its home screen;
 * - `unsupported` — no Web Push here;
 * - `off` — not yet asked; `blocked` — the person said no, in the browser;
 * - `on` — subscribed.
 */
export function stateOf({ apple, standalone, push, permission, subscribed }) {
  if (apple && !standalone) return "install";
  if (!push) return "unsupported";
  if (permission === "denied") return "blocked";
  if (permission === "granted" && subscribed) return "on";
  return "off";
}

/** Whether a browser is an iPhone's or iPad's — an iPad says it is a Mac, with a touch screen. */
export function isApple(userAgent, platform, touchPoints) {
  return /iPad|iPhone|iPod/.test(userAgent) || (platform === "MacIntel" && touchPoints > 1);
}

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/** A base64url string's bytes — what `applicationServerKey` takes. */
export function bytesOf(b64url) {
  const clean = b64url.replace(/=+$/, "").replace(/\+/g, "-").replace(/\//g, "_");
  const out = new Uint8Array(Math.floor((clean.length * 3) / 4));
  let bits = 0;
  let value = 0;
  let at = 0;
  for (const ch of clean) {
    const v = ALPHABET.indexOf(ch);
    if (v < 0) throw new Error(`\`${ch}\` is not base64url`);
    value = (value << 6) | v;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out[at++] = (value >> bits) & 0xff;
    }
  }
  return out;
}

export class Notify {
  /**
   * @param api the app's `Api`
   * @param device what the browser gives: `{apple, standalone, push,
   *   permission(), ask(), registration()}` — `registration()` answers the
   *   service worker's, with its `pushManager`
   */
  constructor(api, device) {
    this.api = api;
    this.device = device;
    /** One of `stateOf`'s states; `null` until it is known. */
    this.state = signal(null);
  }

  /** The browser's subscription, if this device has one. */
  async subscription() {
    if (!this.device.push) return null;
    const reg = await this.device.registration();
    return reg.pushManager.getSubscription();
  }

  /** Learn this device's state. */
  async check() {
    const d = this.device;
    const subscribed = d.push && d.permission() === "granted" ? Boolean(await this.subscription()) : false;
    this.state.value = stateOf({ apple: d.apple, standalone: d.standalone, push: d.push, permission: d.permission(), subscribed });
    return this.state.value;
  }

  /** Ask for notifications on this device, and subscribe it; answers the state after. */
  async enable() {
    const d = this.device;
    if (!d.push || (d.apple && !d.standalone)) return this.check();
    const permission = await d.ask();
    if (permission !== "granted") return this.check();
    const reg = await d.registration();
    const { key } = await this.api.call("GET", "/push/key");
    const sub =
      (await reg.pushManager.getSubscription()) ??
      (await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytesOf(key) }));
    await this.api.call("POST", "/push/subscribe", sub.toJSON());
    this.state.value = "on";
    return "on";
  }

  /** Stop notifications on this device. */
  async disable() {
    const sub = await this.subscription();
    if (sub) {
      await this.api.call("DELETE", "/push/subscribe", { endpoint: sub.endpoint });
      await sub.unsubscribe();
    }
    return this.check();
  }

  /** Stop app `app`'s pushes to this account, or start them again. */
  async mute(app, muted) {
    return this.api.call("PUT", `/notify/${encodeURIComponent(app)}/mute`, { muted });
  }
}

/** What this browser gives `Notify`. */
export function browserDevice() {
  const push = "serviceWorker" in navigator && "PushManager" in globalThis && "Notification" in globalThis;
  return {
    apple: isApple(navigator.userAgent, navigator.platform, navigator.maxTouchPoints ?? 0),
    standalone: matchMedia("(display-mode: standalone)").matches || navigator.standalone === true,
    push,
    permission: () => (push ? Notification.permission : "default"),
    ask: () => Notification.requestPermission(),
    registration: () => navigator.serviceWorker.ready,
  };
}
