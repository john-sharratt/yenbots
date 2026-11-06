//! The check over a whole app: one written as the guide teaches passes every
//! rule at once, and an answer of many refusals is told in order, capped.

use super::files::Files;
use super::*;

/// An app as `docs/framework.md` and `docs/design.md` teach one.
pub fn recipes() -> Files {
    let mut f = Files::default();
    for (path, text) in [
        (
            "apps/recipes/app.yaml",
            "id: recipes\nname: Recipes\nstatus: draft\ntheme:\n  radius: round\ncollections:\n  recipes:\n    fields:\n      \
             title: { type: text, required: true }\n      serves: { type: number, min: 1 }\nschedules:\n  - every: daily 07:30\n    \
             run: morning\n",
        ),
        (
            "apps/recipes/index.html",
            "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <title>Recipes</title>\n  \
             <link rel=\"stylesheet\" href=\"style.css\">\n  <script type=\"module\" src=\"/_/yen.js\"></script>\n  \
             <script type=\"module\" src=\"app.js\"></script>\n</head>\n<body><yen-app></yen-app></body>\n</html>\n",
        ),
        (
            "apps/recipes/app.js",
            "import { defineApp } from \"/_/yen.js\";\nimport \"./pages/home.js\";\n\ndefineApp({ title: \"Recipes\", routes: { \"\": \"recipes-home\" } });\n",
        ),
        ("apps/recipes/style.css", "recipe-card { margin-block: var(--yen-space-2); }\n"),
        (
            "apps/recipes/stores/recipes.js",
            "import { app } from \"/_/yen.js\";\nexport const recipes = app.collection(\"recipes\", { order: \"title\" });\n",
        ),
        (
            "apps/recipes/pages/home.js",
            "import { YenElement, html } from \"/_/yen.js\";\nimport { recipes } from \"../stores/recipes.js\";\nimport \"../components/recipe-card.js\";\n\n\
             export class RecipesHome extends YenElement {\n  render() {\n    return html`<yen-page heading=\"Recipes\">\n      \
             <yen-list .items=${recipes.value} .item=${(r) => html`<recipe-card .recipe=${r}></recipe-card>`}\n        \
             ?loading=${recipes.loading.value} empty=\"No recipes yet\"></yen-list>\n    </yen-page>`;\n  }\n}\n\
             customElements.define(\"recipes-home\", RecipesHome);\n",
        ),
        (
            "apps/recipes/components/recipe-card.js",
            "import { YenElement, html, css } from \"/_/yen.js\";\n\nexport class RecipeCard extends YenElement {\n  \
             static properties = { recipe: { attribute: false } };\n  static styles = css`a { display: block; gap: var(--yen-space-1); }`;\n  \
             render() {\n    return html`<a href=\"r/${this.recipe.id}\"><img src=${this.recipe.photo} alt=\"\">${this.recipe.title}</a>`;\n  }\n}\n\
             customElements.define(\"recipe-card\", RecipeCard);\n",
        ),
        (
            "apps/recipes/workflows/morning.js",
            "export default async function morning(ctx) {\n  ctx.log(\"good morning\");\n}\n",
        ),
    ] {
        f.add(path, text.as_bytes().to_vec());
    }
    f
}

/// **An app written as the guide teaches passes every rule.**
#[test]
fn an_app_written_as_taught_passes() {
    assert_eq!(
        check_app(&recipes(), "recipes", &Before { manifest: None }),
        []
    );
}

/// **Each rule speaks in its own name**, and the answer is in order of path
/// and line.
#[test]
fn every_rule_is_heard_in_order() {
    let mut f = recipes();
    f.add(
        "apps/recipes/style.css",
        b"recipe-card {\n  color: #c2375b;\n  background: url(https://pics.example/a.png);\n}\n"
            .to_vec(),
    );
    f.add("apps/recipes/notes.txt", b"ideas".to_vec());
    f.add(
        "apps/recipes/workflows/morning.js",
        b"export const x = 1;".to_vec(),
    );
    let refused = check_app(&f, "recipes", &Before { manifest: None });
    assert_eq!(
        refused
            .iter()
            .map(|r| (r.rule, r.path.as_str(), r.line))
            .collect::<Vec<_>>(),
        [
            ("layout", "apps/recipes/notes.txt", None),
            ("tokens", "apps/recipes/style.css", Some(2)),
            ("outside", "apps/recipes/style.css", Some(3)),
            ("workflows", "apps/recipes/workflows/morning.js", None),
        ]
    );
}

#[test]
fn a_long_answer_is_capped_and_counted() {
    let many: Vec<Refusal> = (1..=TOLD + 2)
        .map(|n| Refusal::new("tokens", "apps/a/style.css", Some(n), "x"))
        .collect();
    let answer = told(&many);
    assert_eq!(answer.lines().count(), TOLD + 1);
    assert!(
        answer.starts_with("apps/a/style.css:1 x\napps/a/style.css:2 x\n"),
        "{answer}"
    );
    assert!(answer.ends_with("\n… and 2 more"), "{answer}");
}
