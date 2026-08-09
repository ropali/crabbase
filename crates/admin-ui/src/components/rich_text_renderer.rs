use pulldown_cmark::{CowStr, Event, Options, Parser, html};

pub const RICH_TEXT_CONTENT_CLASSES: &str = concat!(
    "text-body-sm text-on-surface leading-7 break-words ",
    "[&_h1]:text-2xl [&_h1]:font-bold [&_h1]:tracking-tight [&_h1]:mt-6 [&_h1]:mb-3 ",
    "[&_h2]:text-xl [&_h2]:font-bold [&_h2]:tracking-tight [&_h2]:mt-5 [&_h2]:mb-3 ",
    "[&_h3]:text-lg [&_h3]:font-semibold [&_h3]:mt-4 [&_h3]:mb-2 ",
    "[&_p]:mb-3 [&_p:last-child]:mb-0 ",
    "[&_ul]:list-disc [&_ul]:pl-5 [&_ul]:my-3 ",
    "[&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:my-3 ",
    "[&_li]:mb-1 ",
    "[&_blockquote]:my-4 [&_blockquote]:border-l-4 [&_blockquote]:border-secondary/35 ",
    "[&_blockquote]:pl-4 [&_blockquote]:italic [&_blockquote]:text-on-surface-variant ",
    "[&_pre]:my-4 [&_pre]:overflow-x-auto [&_pre]:rounded-xl [&_pre]:bg-[#1f2328] ",
    "[&_pre]:p-4 [&_pre]:text-[#f6f8fa] ",
    "[&_code]:rounded [&_code]:bg-surface-container-high [&_code]:px-1 [&_code]:py-0.5 ",
    "[&_code]:font-mono [&_code]:text-[12px] ",
    "[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-inherit ",
    "[&_a]:text-secondary [&_a]:underline [&_a]:underline-offset-2 ",
    "[&_hr]:my-5 [&_hr]:border-outline-variant ",
    "[&_table]:my-4 [&_table]:w-full [&_table]:border-collapse ",
    "[&_th]:border [&_th]:border-outline-variant [&_th]:bg-surface-container-low ",
    "[&_th]:px-3 [&_th]:py-2 [&_th]:text-left ",
    "[&_td]:border [&_td]:border-outline-variant [&_td]:px-3 [&_td]:py-2"
);

pub fn rich_text_to_html(content: &str) -> String {
    if looks_like_legacy_html(content) {
        sanitize_html(content)
    } else {
        markdown_to_html(content)
    }
}

fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown, options).map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => {
            Event::Text(CowStr::Boxed(raw.into_string().into_boxed_str()))
        }
        other => other,
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    sanitize_html(&html_output)
}

fn sanitize_html(raw_html: &str) -> String {
    ammonia::clean(raw_html)
}

fn looks_like_legacy_html(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('<') && trimmed.contains('>')
}

#[cfg(test)]
mod tests {
    use super::rich_text_to_html;

    #[test]
    fn markdown_is_rendered() {
        let html = rich_text_to_html("# Heading\n\nA **bold** line.");

        assert!(html.contains("<h1>Heading</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn markdown_links_are_sanitized() {
        let html = rich_text_to_html("[x](javascript:alert(1))");

        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn legacy_html_is_preserved_but_sanitized() {
        let html = rich_text_to_html("<h1>Hello</h1><script>alert(1)</script>");

        assert!(html.contains("<h1>Hello</h1>"));
        assert!(!html.contains("<script>"));
    }
}
