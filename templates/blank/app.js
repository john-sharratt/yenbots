import { defineApp } from "/_/yen.js";
import "./pages/home.js";

defineApp({
  title: "Notes",
  routes: {
    "": "blank-home",
  },
});
