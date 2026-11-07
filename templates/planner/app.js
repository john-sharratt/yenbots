import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/new.js";
import "./pages/plan.js";

defineApp({
  title: "Planner",
  routes: {
    "": "planner-home",
    "new": "planner-new",
    "p/:id": "planner-plan",
  },
});
