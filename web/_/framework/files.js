// `app.files` (§6.1): a file kept for the app — a photo taken, a PDF chosen
// — answered with its id, which a record's `file` field holds, and the
// addresses of it and its thumbnail.
//
//   const { id, thumb } = await app.files.upload(input.files[0]);

import { ApiError, Offline } from "./api.js";

export class Files {
  /** @param api the app's `Api` */
  constructor(api) {
    this.api = api;
  }

  /** Keep `blob` (a `File` or `Blob`), named `name` or its own name. */
  async upload(blob, { name } = {}) {
    const called = encodeURIComponent(name ?? blob.name ?? "file");
    let res;
    try {
      res = await this.api.fetch(`/api/v1/files/${encodeURIComponent(this.api.app)}?name=${called}`, {
        method: "POST",
        credentials: "same-origin",
        headers: { "X-Yen-App": this.api.app, "Content-Type": blob.type || "application/octet-stream" },
        body: blob,
      });
    } catch (e) {
      throw new Offline(e);
    }
    const text = await res.text();
    let json = null;
    try {
      json = JSON.parse(text);
    } catch {
      json = { error: `http_${res.status}`, detail: text.slice(0, 200) };
    }
    if (!res.ok) throw new ApiError(res.status, json);
    return json;
  }

  /** The address of file `id`, or of its thumbnail. */
  url(id, { thumb = false } = {}) {
    return `/api/v1/files/${encodeURIComponent(this.api.app)}/${encodeURIComponent(id)}${thumb ? "/thumb" : ""}`;
  }
}
