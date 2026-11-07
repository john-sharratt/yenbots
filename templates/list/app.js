import { defineApp } from "/_/yen.js";
import "./pages/home.js";
import "./pages/item.js";

defineApp({
  title: "Shopping",
  routes: {
    "": "list-home",
    "i/:id": "list-item-page",
  },
});
