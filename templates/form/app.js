import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/summary.js";

defineApp({
  title: "Party poll",
  routes: {
    "": "form-home",
    "summary": "form-summary",
  },
});
