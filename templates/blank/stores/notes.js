// Each person's own notes, newest first.
import { app } from "/_/yen.js";

export const notes = app.collection("notes", { order: "-created_ms" });
