import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/person-new.js";
import "./pages/person.js";
import "./pages/matter-new.js";
import "./pages/matter.js";
import "./pages/ask.js";

defineApp({
  title: "Employee Lifecycle",
  routes: {
    "": "lifecycle-home",
    "new": "person-new",
    "ask": "lifecycle-ask",
    "p/:id": "person-page",
    "matter/:person": "matter-new",
    "m/:id": "matter-page",
  },
});
