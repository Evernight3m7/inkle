use pulldown_cmark::{html, Options, Parser};

pub fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    // Strip dangerous HTML (scripts, event handlers, etc.) while preserving
    // safe tags and attributes that Markdown may produce.
    ammonia::Builder::default()
        .add_generic_attributes(&["class", "id"])
        .clean(&html_output)
        .to_string()
}
