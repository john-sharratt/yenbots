import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/new.js";

defineApp({
  title: "Reminders",
  routes: {
    "": "reminders-home",
    "new": "reminders-new",
  },
});
