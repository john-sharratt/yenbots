import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/kept.js";

defineApp({
  title: "Pictures",
  routes: {
    "": "picture-home",
    "kept": "picture-kept",
  },
});
