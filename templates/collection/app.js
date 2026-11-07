import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/new.js";
import "./pages/item.js";

defineApp({
  title: "Recipes",
  routes: {
    "": "collection-home",
    "new": "collection-new",
    "i/:id": "collection-item",
  },
});
