import { defineApp } from "/_/yen.js";
import "./pages/home.js";

defineApp({
  title: "Homework helper",
  routes: {
    "": "assistant-home",
  },
});
