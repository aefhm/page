use anyhow::{Context, Result};
use jotdown::Render;

const SITE_URL: &str = "https://xizhang.page";
const SITE_NAME: &str = "Field Bobbin";
const SITE_DESCRIPTION: &str =
    "Personal site by Xi Zhang with recipes, readings, writings, and an about me page.";

struct RecipeSummary {
    slug: String,
    name: String,
}

struct PostSummary {
    slug: String,
    title: String,
    date_published: String,
}

fn main() -> Result<()> {
    rebuild_public_dir()?;

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

    sort_posts_chronologically(&mut posts);

    let writings_body = render_writings_index(&posts);
    let writings_html = render_layout("Writings", &writings_body, None);

    std::fs::write("public/writings/index.html", writings_html)?;

    let llms_text = render_llms_txt(&recipes, &posts);
    std::fs::write("public/llms.txt", llms_text)?;

    Ok(())
}

fn rebuild_public_dir() -> Result<()> {
    let public = std::path::Path::new("public");

    if public.exists() {
        std::fs::remove_dir_all(public)?;
    }

    std::fs::create_dir("public")?;
    std::fs::create_dir("public/fonts")?;
    std::fs::create_dir("public/images")?;
    std::fs::create_dir("public/recipes")?;
    std::fs::create_dir("public/writings")?;

    std::fs::copy("static/style.css", "public/style.css")?;
    std::fs::copy("static/robots.txt", "public/robots.txt")?;

    for entry in std::fs::read_dir("static/fonts")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), public.join("fonts").join(entry.file_name()))?;
        }
    }
    for entry in std::fs::read_dir("static/images")? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), public.join("images").join(entry.file_name()))?;
        }
    }

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
        let display_date = display_writing_date(&post.date_published);
        let full_date = full_writing_date(&post.date_published);

        list_html.push_str(&format!(
            r#"        <li><a href="./{}.html">{}</a><time class="writing-date" datetime="{}" aria-label="{}" title="{}">{}</time></li>
"#,
            post.slug, post.title, post.date_published, full_date, full_date, display_date
        ));
    }

    format!(
        r#"    <article class="writings">
    <section>
      <h2>Writings</h2>
      <ul class="writing-list">
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

    let slug = writing_meta_value(meta_line, "slug")
        .context("writing missing slug identifier")?
        .to_string();

    let date_published = writing_meta_value(meta_line, "datePublished")
        .context("writing missing datePublished identifier")?
        .to_string();

    if !is_iso_date(&date_published) {
        anyhow::bail!("writing datePublished must be a valid YYYY-MM-DD date");
    }

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

    Ok(PostSummary {
        slug,
        title,
        date_published,
    })
}

fn writing_meta_value<'a>(meta_line: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field}=\"");
    let value_start = meta_line.find(&prefix)? + prefix.len();
    let rest = &meta_line[value_start..];

    rest.split('"').next()
}

fn is_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();

    if !(bytes.len() == 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit))
    {
        return false;
    }

    let Ok(year) = date[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = date[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = date[8..10].parse::<u32>() else {
        return false;
    };

    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };

    day > 0 && day <= max_day
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn display_writing_date(date: &str) -> String {
    let Some((month, _, year)) = writing_date_parts(date, MonthNameStyle::Short) else {
        return date.to_string();
    };

    format!("{month} {year}")
}

fn full_writing_date(date: &str) -> String {
    let Some((month, day, year)) = writing_date_parts(date, MonthNameStyle::Long) else {
        return date.to_string();
    };

    format!("{month} {day}, {year}")
}

enum MonthNameStyle {
    Short,
    Long,
}

fn writing_date_parts(
    date: &str,
    month_name_style: MonthNameStyle,
) -> Option<(&'static str, &str, &str)> {
    if !is_iso_date(date) {
        return None;
    }

    let month = match (month_name_style, &date[5..7]) {
        (MonthNameStyle::Short, "01") => "Jan",
        (MonthNameStyle::Short, "02") => "Feb",
        (MonthNameStyle::Short, "03") => "Mar",
        (MonthNameStyle::Short, "04") => "Apr",
        (MonthNameStyle::Short, "05") => "May",
        (MonthNameStyle::Short, "06") => "Jun",
        (MonthNameStyle::Short, "07") => "Jul",
        (MonthNameStyle::Short, "08") => "Aug",
        (MonthNameStyle::Short, "09") => "Sep",
        (MonthNameStyle::Short, "10") => "Oct",
        (MonthNameStyle::Short, "11") => "Nov",
        (MonthNameStyle::Short, "12") => "Dec",
        (MonthNameStyle::Long, "01") => "January",
        (MonthNameStyle::Long, "02") => "February",
        (MonthNameStyle::Long, "03") => "March",
        (MonthNameStyle::Long, "04") => "April",
        (MonthNameStyle::Long, "05") => "May",
        (MonthNameStyle::Long, "06") => "June",
        (MonthNameStyle::Long, "07") => "July",
        (MonthNameStyle::Long, "08") => "August",
        (MonthNameStyle::Long, "09") => "September",
        (MonthNameStyle::Long, "10") => "October",
        (MonthNameStyle::Long, "11") => "November",
        (MonthNameStyle::Long, "12") => "December",
        _ => return None,
    };
    let day = date[8..10].trim_start_matches('0');
    let year = &date[0..4];

    Some((month, day, year))
}

fn sort_posts_chronologically(posts: &mut [PostSummary]) {
    posts.sort_by(|a, b| {
        b.date_published
            .cmp(&a.date_published)
            .then_with(|| a.title.cmp(&b.title))
    });
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
      <h1>
      <a class="site-title" href="/index.html" aria-label="Field Bobbin Home">
        <span class="field">Field</span> <span class="accent">Bobbin</span>
      </a>
      </h1>
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

fn render_llms_txt(recipes: &[RecipeSummary], posts: &[PostSummary]) -> String {
    let mut recipe_lines = String::new();

    for recipe in recipes {
        recipe_lines.push_str(&format!(
            "- [{}]({}/recipes/{}.html) - [JSON-LD]({}/recipes/{}.jsonld)\n",
            recipe.name, SITE_URL, recipe.slug, SITE_URL, recipe.slug
        ));
    }

    let mut post_lines = String::new();

    for post in posts {
        post_lines.push_str(&format!(
            "- [{}]({}/writings/{}.html)\n",
            post.title, SITE_URL, post.slug
        ));
    }

    format!(
        r#"# {SITE_NAME}

{SITE_DESCRIPTION}
            
## Pages

- [About]({SITE_URL}/index.html)
- [Recipes]({SITE_URL}/recipes/)
- [Writings]({SITE_URL}/writings/)
- [Readings]({SITE_URL}/readings/index.html)

## Recipes

Recipe pages include embedded Schema.org JSON-LD. Sidecar JSON-LD files are available next to each recipe page.

{recipe_lines}

## Writings

{post_lines}

## Readings

- [Readings]({SITE_URL}/readings/index.html)
        "#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_writing_metadata_regardless_of_field_order() {
        let meta = r#"{inLanguage="en" datePublished="2026-06-12" slug="example"}"#;

        assert_eq!(writing_meta_value(meta, "slug"), Some("example"));
        assert_eq!(
            writing_meta_value(meta, "datePublished"),
            Some("2026-06-12")
        );
    }

    #[test]
    fn sorts_posts_newest_first_then_by_title() {
        let mut posts = vec![
            PostSummary {
                slug: "older".to_string(),
                title: "Older".to_string(),
                date_published: "2025-01-01".to_string(),
            },
            PostSummary {
                slug: "newer-b".to_string(),
                title: "B".to_string(),
                date_published: "2026-01-01".to_string(),
            },
            PostSummary {
                slug: "newer-a".to_string(),
                title: "A".to_string(),
                date_published: "2026-01-01".to_string(),
            },
        ];

        sort_posts_chronologically(&mut posts);

        assert_eq!(
            posts
                .iter()
                .map(|post| post.slug.as_str())
                .collect::<Vec<_>>(),
            ["newer-a", "newer-b", "older"]
        );
    }

    #[test]
    fn validates_iso_date_shape() {
        assert!(is_iso_date("2026-06-12"));
        assert!(is_iso_date("2024-02-29"));
        assert!(!is_iso_date("2026-6-12"));
        assert!(!is_iso_date("June 12, 2026"));
        assert!(!is_iso_date("2026-02-29"));
        assert!(!is_iso_date("2026-13-01"));
    }

    #[test]
    fn displays_iso_dates_as_human_readable_dates() {
        assert_eq!(display_writing_date("2026-06-12"), "Jun 2026");
        assert_eq!(display_writing_date("2026-05-01"), "May 2026");
        assert_eq!(full_writing_date("2026-06-12"), "June 12, 2026");
        assert_eq!(full_writing_date("2026-05-01"), "May 1, 2026");
    }

    #[test]
    fn renders_published_dates_in_writings_index() {
        let html = render_writings_index(&[PostSummary {
            slug: "example".to_string(),
            title: "Example".to_string(),
            date_published: "2026-06-12".to_string(),
        }]);

        assert!(
            html.contains(
                r#"<time class="writing-date" datetime="2026-06-12" aria-label="June 12, 2026" title="June 12, 2026">Jun 2026</time>"#
            )
        );
    }
}
