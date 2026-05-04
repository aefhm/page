use anyhow::{Context, Result};
use jotdown::Render;

struct RecipeSummary {
    slug: String,
    name: String,
}

struct PostSummary {
    slug: String,
    title: String,
}

fn main() -> Result<()> {
    std::fs::create_dir_all("public/recipes")?;
    std::fs::create_dir_all("public/writings")?;

    build_page("pages/index.html", "public/index.html", "Xi")?;
    build_page(
        "pages/readings/index.html",
        "public/readings/index.html",
        "Readings",
    )?;

    let mut recipes = Vec::new();

    for entry in std::fs::read_dir("recipes")? {
        let path = entry?.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonld") {
            continue;
        }

        let recipe = build_recipe(&path)?;
        recipes.push(recipe);
    }

    recipes.sort_by(|a, b| a.name.cmp(&b.name));

    let index_body = render_recipes_index(&recipes);
    let index_jsonld = render_recipes_index_jsonld(&recipes)?;
    let index_html = render_layout("Recipes", &index_body, Some(&index_jsonld));

    std::fs::write("public/recipes/index.html", index_html)?;

    let mut posts = Vec::new();

    for entry in std::fs::read_dir("writings")? {
        let path = entry?.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("dj") {
            continue;
        }

        let post = build_writing(&path)?;
        posts.push(post);
    }

    posts.sort_by(|a, b| a.title.cmp(&b.title));

    let writings_body = render_writings_index(&posts);
    let writings_html = render_layout("Writings", &writings_body, None);

    std::fs::write("public/writings/index.html", writings_html)?;

    Ok(())
}

fn build_page(source: &str, output: &str, title: &str) -> Result<()> {
    let body = std::fs::read_to_string(source)?;
    let html = render_layout(title, &body, None);

    if let Some(parent) = std::path::Path::new(output).parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output, html)?;

    Ok(())
}

fn render_recipes_index(recipes: &[RecipeSummary]) -> String {
    let mut list_html = String::new();

    for recipe in recipes {
        list_html.push_str(&format!(
            r#"
                <li><a href="./{}.html">{}</a></li>
            "#,
            recipe.slug, recipe.name
        ));
    }

    format!(
        r#"
            <article class="recipes">
            <section>
            <h2>Recipes</h2>
            <ul>
            {list_html}
            </ul>
            </section>
            </article>
        "#
    )
}

fn render_recipes_index_jsonld(recipes: &[RecipeSummary]) -> Result<String> {
    let mut items = Vec::new();

    for (index, recipe) in recipes.iter().enumerate() {
        items.push(serde_json::json!({
            "@type": "ListItem",
            "position": index + 1,
            "url": format!("./{}.html", recipe.slug),
            "name": recipe.name,
        }));
    }

    let jsonld = serde_json::json!(
        {
            "@context": "https://schema.org",
            "@type": "ItemList",
            "name": "Recipes",
            "itemListElement": items,
        }
    );

    Ok(serde_json::to_string_pretty(&jsonld)?)
}

fn build_recipe(path: &std::path::Path) -> Result<RecipeSummary> {
    let raw = std::fs::read_to_string(path)?;
    let mut recipe: serde_json::Value = serde_json::from_str(&raw)?;

    let slug = recipe
        .get("slug")
        .and_then(|value| value.as_str())
        .context("recipe missing string field: slug")?
        .to_string();

    let name = recipe
        .get("name")
        .and_then(|value| value.as_str())
        .context("recipe missing string field: name")?
        .to_string();

    recipe
        .as_object_mut()
        .context("recipe JSON-LD must be an object")?
        .remove("slug");

    let ingredients = recipe
        .get("recipeIngredient")
        .and_then(|value| value.as_array())
        .context("recipe missing array field: recipeIngredient")?;

    let mut ingredients_html = String::new();

    for ingredient in ingredients {
        let ingredient = ingredient
            .as_str()
            .context("recipeIngredient must only contain strings")?;

        ingredients_html.push_str(&format!("  <li>{ingredient}</li>\n"));
    }

    let instructions = recipe
        .get("recipeInstructions")
        .and_then(|value| value.as_array())
        .context("recipe missing array field: recipeInstructions")?;

    let mut steps_html = String::new();

    for instruction in instructions {
        let instruction_type = instruction
            .get("@type")
            .and_then(|value| value.as_str())
            .context("recipe instruction missing string field: @type")?;

        if instruction_type != "HowToStep" {
            continue;
        }

        let text = instruction
            .get("text")
            .and_then(|value| value.as_str())
            .context("recipe instruction missing string field: text")?;

        let step_image_html = if let Some(image) = instruction.get("image") {
            let url = image
                .get("contentUrl")
                .and_then(|value| value.as_str())
                .context("step image missing string field: contentUrl")?;

            let caption = image
                .get("caption")
                .and_then(|value| value.as_str())
                .unwrap_or("");

            format!(
                r#"
      <figure class="step-photo">
        <img src="{url}" alt="{caption}" loading="lazy">
        <figcaption>{caption}</figcaption>
      </figure>"#
            )
        } else {
            String::new()
        };

        steps_html.push_str(&format!("    <li><p>{text}</p>{step_image_html}</li>\n"));
    }

    let mut notes_html = String::new();

    for instruction in instructions {
        let instruction_type = instruction
            .get("@type")
            .and_then(|value| value.as_str())
            .context("recipe instruction missing string field: @type")?;

        if instruction_type != "HowToTip" {
            continue;
        }

        let text = instruction
            .get("text")
            .and_then(|value| value.as_str())
            .context("recipe instruction missing string field: text")?;

        notes_html.push_str(&format!("    <p>{text}</p>\n"));
    }

    let notes_section = if notes_html.is_empty() {
        String::new()
    } else {
        format!(
            r#"    <section class="recipe-notes">
            <h3>Notes</h3>
            {notes_html} </section>
            "#
        )
    };

    let output_path = format!("public/recipes/{slug}.jsonld");

    let public_jsonld = serde_json::to_string_pretty(&recipe)?;

    std::fs::write(&output_path, &public_jsonld)
        .with_context(|| format!("writing {output_path}"))?;

    let hero_html = if let Some(image) = recipe.get("image") {
        let url = image
            .get("url")
            .and_then(|value| value.as_str())
            .context("recipe image missing string field: url")?;

        let caption = image
            .get("caption")
            .and_then(|value| value.as_str())
            .unwrap_or("");

        format!(
            r#"    <figure class="recipe-hero">
      <img src="{url}" alt="{caption}" loading="lazy">
    </figure>
"#
        )
    } else {
        String::new()
    };

    let body = format!(
        r#"   <article class="recipes">
        <section>
    <h2>{name}</h2>
    {hero_html}
    <h3>Ingredients</h3>
    <ul>
    {ingredients_html}    </ul>
    <h3>Steps</h3>
    <ol>
    {steps_html} </ol>
    </section>
    {notes_section} </article>"#
    );

    let html = render_layout(&name, &body, Some(&public_jsonld));

    let html_output_path = format!("public/recipes/{slug}.html");

    std::fs::write(&html_output_path, html)
        .with_context(|| format!("writing {html_output_path}"))?;

    Ok(RecipeSummary { slug, name })
}

fn render_writings_index(posts: &[PostSummary]) -> String {
    let mut list_html = String::new();

    for post in posts {
        list_html.push_str(&format!(
            r#"        <li><a href="./{}.html">{}</a></li>
"#,
            post.slug, post.title
        ));
    }

    format!(
        r#"    <article class="writings">
    <section>
      <h2>Writings</h2>
      <ul>
{list_html}        </ul>
    </section>
  </article>"#
    )
}

fn build_writing(path: &std::path::Path) -> Result<PostSummary> {
    let source = std::fs::read_to_string(path)?;

    let mut lines = source.lines();

    let meta_line = lines.next().context("writing missing meta line")?;

    let body_src = lines.collect::<Vec<_>>().join("\n");

    let slug = meta_line
        .strip_prefix("{slug=\"")
        .and_then(|rest| rest.split('"').next())
        .context("writing missing slug identifier")?
        .to_string();

    let title = body_src
        .lines()
        .find_map(|line| line.strip_prefix("## "))
        .context("writing missing title heading")?
        .to_string();

    let body_html = render_djot(&body_src)?;

    let post_body = format!(
        r#"<article class="writings">
        {body_html}
        </article>
       "#
    );

    let html = render_layout(&title, &post_body, None);

    std::fs::write(format!("public/writings/{slug}.html"), html)?;

    Ok(PostSummary { slug, title })
}

fn render_layout(title: &str, body: &str, jsonld: Option<&str>) -> String {
    let jsonld_script = match jsonld {
        Some(jsonld) => format!(r#"<script type="application/ld+json"> {jsonld}</script>"#),
        None => String::new(),
    };

    format!(
        r#"<!doctype html>
  <html lang="en-US">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <link rel="stylesheet" type="text/css" href="/style.css">
    {jsonld_script}
  </head>
  <body>
    <header>
      <h1><span class="field">Field</span> <span class="accent">Bobbin</span></h1>
    </header>
    <nav>
      <ul>
        <li><a href="/readings">Readings</a></li>
        <li><a href="/writings">Writings</a></li>
        <li><a href="/recipes">Recipes</a></li>
        <li><a href="/forms/">Forms</a></li>
        <li><a href="/index.html">About</a></li>
      </ul>
    </nav>
    <main>
  {body}
    </main>
  </body>
  </html>
  "#
    )
}

fn render_djot(src: &str) -> Result<String> {
    let mut out = String::new();

    jotdown::html::Renderer::default().push(jotdown::Parser::new(src), &mut out)?;

    Ok(out)
}
