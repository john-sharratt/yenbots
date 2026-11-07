import { defineApp } from "/_/yen.js";
import "./pages/home.js";

defineApp({
  title: "Savings",
  routes: {
    "": "tracker-home",
  },
});
