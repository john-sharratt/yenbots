import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/new.js";

defineApp({
  title: "Today",
  routes: {
    "": "digest-home",
    "new": "digest-new",
  },
});
