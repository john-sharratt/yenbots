import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/new.js";
import "./pages/entry.js";

defineApp({
  title: "Journal",
  routes: {
    "": "journal-home",
    "new": "journal-new",
    "e/:id": "journal-entry",
  },
});
