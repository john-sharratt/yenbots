# Apps

One folder per app, named for its id: `apps/<id>/`. The site reads them from
its live checkout and serves each at `/a/<id>/`; a push to `main` is live in
about a second.

An app's shape, which the check holds every app to (`docs/check.md`):

```text
<id>/
  app.yaml      the manifest: name, collections, theme, tile (docs/framework.md)
  index.html  app.js  style.css
  pages/        one file per page
  components/   one file per component, named for its tag
  stores/       one file per kind of data
  workflows/    the server's functions
```

A new app starts as a copy of the closest template (`templates/`), built to
the app-building guide (`docs/building-an-app.md`).
