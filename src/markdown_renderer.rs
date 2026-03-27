use gtk4::prelude::*;
use gtk4::{TextBuffer, TextTag, TextIter, TextView};
use gtk4::glib;
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, Options, Alignment};
use crate::config::Config;

pub struct MarkdownRenderer {
    // Text formatting tags
    h1_tag: TextTag,
    h2_tag: TextTag,
    h3_tag: TextTag,
    h4_tag: TextTag,
    h5_tag: TextTag,
    h6_tag: TextTag,
    bold_tag: TextTag,
    italic_tag: TextTag,
    code_tag: TextTag,
    code_block_tag: TextTag,
    link_tag: TextTag,
    quote_tag: TextTag,
    think_tag: TextTag,
    
    // State for nested formatting
    format_stack: Vec<TextTag>,
    // Track if tags are already setup
    tags_setup: bool,
    // State for streaming think tag processing
    in_think_tag: bool,
    // Clickable link tracking
    pub link_ranges: Vec<(gtk4::TextMark, gtk4::TextMark, String)>,
    pending_link: Option<(gtk4::TextMark, String)>,
    // Text view reference for embedding table widgets
    text_view: Option<TextView>,
    // Table rendering state
    in_table: bool,
    in_table_head: bool,
    in_table_cell: bool,
    current_cell_text: String,
    current_row: Vec<(String, bool)>,
    table_data: Vec<Vec<(String, bool)>>,
    table_alignments: Vec<Alignment>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            h1_tag: TextTag::new(Some("h1")),
            h2_tag: TextTag::new(Some("h2")),
            h3_tag: TextTag::new(Some("h3")),
            h4_tag: TextTag::new(Some("h4")),
            h5_tag: TextTag::new(Some("h5")),
            h6_tag: TextTag::new(Some("h6")),
            bold_tag: TextTag::new(Some("bold")),
            italic_tag: TextTag::new(Some("italic")),
            code_tag: TextTag::new(Some("code")),
            code_block_tag: TextTag::new(Some("code_block")),
            link_tag: TextTag::new(Some("link")),
            quote_tag: TextTag::new(Some("quote")),
            think_tag: TextTag::new(Some("think")),
            format_stack: Vec::new(),
            tags_setup: false,
            in_think_tag: false,
            link_ranges: Vec::new(),
            pending_link: None,
            text_view: None,
            in_table: false,
            in_table_head: false,
            in_table_cell: false,
            current_cell_text: String::new(),
            current_row: Vec::new(),
            table_data: Vec::new(),
            table_alignments: Vec::new(),
        }
    }

    pub fn set_text_view(&mut self, tv: TextView) {
        self.text_view = Some(tv);
    }
    
    pub fn setup_tags(&mut self, buffer: &TextBuffer, config: &Config) {
        if self.tags_setup {
            return; // Tags already setup
        }
        
        // Configure heading tags
        self.h1_tag.set_weight(700);
        self.h1_tag.set_scale(2.0);
        self.h1_tag.set_property("pixels-above-lines", 12);
        self.h1_tag.set_property("pixels-below-lines", 6);
        
        self.h2_tag.set_weight(700);
        self.h2_tag.set_scale(1.5);
        self.h2_tag.set_property("pixels-above-lines", 10);
        self.h2_tag.set_property("pixels-below-lines", 5);
        
        self.h3_tag.set_weight(700);
        self.h3_tag.set_scale(1.3);
        self.h3_tag.set_property("pixels-above-lines", 8);
        self.h3_tag.set_property("pixels-below-lines", 4);
        
        self.h4_tag.set_weight(700);
        self.h4_tag.set_scale(1.1);
        self.h4_tag.set_property("pixels-above-lines", 6);
        self.h4_tag.set_property("pixels-below-lines", 3);
        
        self.h5_tag.set_weight(700);
        self.h5_tag.set_scale(1.0);
        self.h5_tag.set_property("pixels-above-lines", 4);
        self.h5_tag.set_property("pixels-below-lines", 2);
        
        self.h6_tag.set_weight(600);
        self.h6_tag.set_scale(0.9);
        self.h6_tag.set_property("pixels-above-lines", 3);
        self.h6_tag.set_property("pixels-below-lines", 2);
        
        // Configure text formatting tags
        self.bold_tag.set_weight(700);
        self.italic_tag.set_style(gtk4::pango::Style::Italic);
        
        // Configure code tags with config colors
        self.code_tag.set_family(Some(&config.ui.code_font_family));
        if let Ok(bg_color) = parse_color(&config.colors.code_background) {
            self.code_tag.set_background_rgba(Some(&bg_color));
        }
        if let Ok(fg_color) = parse_color(&config.colors.code_text) {
            self.code_tag.set_foreground_rgba(Some(&fg_color));
        }

        self.code_block_tag.set_family(Some(&config.ui.code_font_family));
        if let Ok(bg_color) = parse_color(&config.colors.code_background) {
            self.code_block_tag.set_background_rgba(Some(&bg_color));
        }
        if let Ok(fg_color) = parse_color(&config.colors.code_text) {
            self.code_block_tag.set_foreground_rgba(Some(&fg_color));
        }
        self.code_block_tag.set_property("left-margin", 20);
        self.code_block_tag.set_property("right-margin", 20);
        self.code_block_tag.set_property("pixels-above-lines", 8);
        self.code_block_tag.set_property("pixels-below-lines", 8);
        
        // Configure link tag with config color
        if let Ok(link_color) = parse_color(&config.colors.link_text) {
            self.link_tag.set_foreground_rgba(Some(&link_color));
        }
        self.link_tag.set_underline(gtk4::pango::Underline::Single);
        
        // Configure quote tag
        self.quote_tag.set_foreground_rgba(Some(&gtk4::gdk::RGBA::new(0.5, 0.5, 0.5, 1.0)));
        self.quote_tag.set_style(gtk4::pango::Style::Italic);
        self.quote_tag.set_property("left-margin", 20);
        self.quote_tag.set_property("pixels-above-lines", 4);
        self.quote_tag.set_property("pixels-below-lines", 4);
        
        // Configure think tag with config color
        if let Ok(think_color) = parse_color(&config.colors.think_text) {
            self.think_tag.set_foreground_rgba(Some(&think_color));
        }
        self.think_tag.set_style(gtk4::pango::Style::Italic);
        self.think_tag.set_scale(0.9);
        self.think_tag.set_property("left-margin", 15);
        self.think_tag.set_property("right-margin", 15);
        self.think_tag.set_property("pixels-above-lines", 6);
        self.think_tag.set_property("pixels-below-lines", 6);
        
        // Add tags to buffer
        let tag_table = buffer.tag_table();
        let tags = [
            &self.h1_tag, &self.h2_tag, &self.h3_tag, 
            &self.h4_tag, &self.h5_tag, &self.h6_tag,
            &self.bold_tag, &self.italic_tag, &self.code_tag,
            &self.code_block_tag, &self.link_tag, &self.quote_tag,
            &self.think_tag,
        ];
        
        for tag in tags {
            if let Some(tag_name) = tag.name() {
                if tag_table.lookup(&tag_name).is_none() {
                    tag_table.add(tag);
                }
            }
        }
        
        self.tags_setup = true;
    }
    
    
    /// Render markdown starting at the given iterator position without clearing the buffer
    pub fn render_markdown_at_iter(&mut self, buffer: &TextBuffer, markdown_text: &str, iter: &mut TextIter, config: &Config) {
        // Ensure tags are setup with current config
        self.setup_tags(buffer, config);
        
        // Process text for think tags during streaming
        let processed_text = self.process_streaming_text(buffer, markdown_text, iter);
        
        if !processed_text.is_empty() {
            // Configure pulldown-cmark options
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_TASKLISTS);
            options.insert(Options::ENABLE_FOOTNOTES);
            options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
            
            let parser = Parser::new_ext(&processed_text, options);
            
            for event in parser {
                self.process_event(buffer, iter, event);
            }
        }
    }
    
    /// Process text for streaming, handling think tags in real-time.
    ///
    /// Delegates detection to [`parse_think_segments`] and handles GTK insertions per segment.
    fn process_streaming_text(&mut self, buffer: &TextBuffer, text: &str, iter: &mut TextIter) -> String {
        let mut result = String::new();
        for segment in parse_think_segments(text, &mut self.in_think_tag) {
            match segment {
                StreamSegment::Normal(s) => result.push_str(&s),
                StreamSegment::ThinkStart => buffer.insert(iter, "\n💭 "),
                StreamSegment::Think(s) => buffer.insert_with_tags(iter, &s, &[&self.think_tag]),
                StreamSegment::ThinkEnd => buffer.insert(iter, "\n\n"),
            }
        }
        result
    }
    
   fn process_event(&mut self, buffer: &TextBuffer, iter: &mut TextIter, event: Event) {
        match event {
            Event::Start(tag) => {
                self.handle_start_tag(buffer, iter, tag);
            }
            Event::End(tag_end) => {
                self.handle_end_tag(buffer, iter, tag_end);
            }
            Event::Text(text) => {
                if self.in_table_cell {
                    self.current_cell_text.push_str(&text);
                } else {
                    self.insert_text(buffer, iter, &text);
                }
            }
            Event::Code(code) => {
                let active_tags: Vec<&TextTag> = self.format_stack.iter().collect();
                let mut all_tags = vec![&self.code_tag];
                all_tags.extend(active_tags);
                buffer.insert_with_tags(iter, &code, &all_tags);
            }
            Event::Html(html) => {
                // Skip HTML for security - or you could sanitize it
                buffer.insert(iter, &format!("[HTML: {}]", html));
            }
            Event::FootnoteReference(name) => {
                buffer.insert(iter, &format!("[^{}]", name));
            }
            Event::SoftBreak => {
                buffer.insert(iter, " ");
            }
            Event::HardBreak => {
                buffer.insert(iter, "\n");
            }
            Event::Rule => {
                buffer.insert(iter, "\n────────────────────────────────────────\n");
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "☑ " } else { "☐ " };
                buffer.insert(iter, marker);
            }
            Event::InlineMath(math) => {
                // For inline math, you might want to render it differently
                buffer.insert(iter, &format!("${}$", math));
            }
            Event::DisplayMath(math) => {
                // For display math, typically rendered in its own line
                buffer.insert(iter, &format!("\n$$\n{}\n$$\n", math));
            }
            Event::InlineHtml(html) => {
                // Similar to HTML handling, you might want to skip or sanitize
                buffer.insert(iter, &format!("[InlineHTML: {}]", html));
            }
        }
    }
    
    fn handle_start_tag(&mut self, buffer: &TextBuffer, iter: &mut TextIter, tag: Tag) {
        let format_tag = match tag {
            Tag::Heading { level, .. } => {
                // Add some spacing before headings if not at start
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                match level {
                    HeadingLevel::H1 => Some(&self.h1_tag),
                    HeadingLevel::H2 => Some(&self.h2_tag),
                    HeadingLevel::H3 => Some(&self.h3_tag),
                    HeadingLevel::H4 => Some(&self.h4_tag),
                    HeadingLevel::H5 => Some(&self.h5_tag),
                    HeadingLevel::H6 => Some(&self.h6_tag),
                }
            }
            Tag::Paragraph => {
                // Add spacing before paragraphs if not at start
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                None
            }
            Tag::Emphasis => Some(&self.italic_tag),
            Tag::Strong => Some(&self.bold_tag),
            Tag::Link { dest_url, .. } => {
                let mark = buffer.create_mark(None, iter, true); // left-gravity
                self.pending_link = Some((mark, dest_url.to_string()));
                Some(&self.link_tag)
            }
            Tag::CodeBlock(_) => {
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                Some(&self.code_block_tag)
            }
            Tag::BlockQuote(_) => {
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                Some(&self.quote_tag)
            }
            Tag::List(_) => {
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                None
            }
            Tag::Item => {
                buffer.insert(iter, "• ");
                None
            }
            Tag::Table(alignments) => {
                if iter.offset() > 0 {
                    buffer.insert(iter, "\n\n");
                }
                self.in_table = true;
                self.table_data.clear();
                self.table_alignments = alignments.to_vec();
                None
            }
            Tag::TableHead => {
                self.in_table_head = true;
                None
            }
            Tag::TableRow => {
                self.current_row.clear();
                None
            }
            Tag::TableCell => {
                self.in_table_cell = true;
                self.current_cell_text.clear();
                None
            }
            _ => None,
        };

        if let Some(tag_ref) = format_tag {
            self.format_stack.push(tag_ref.clone());
        }
    }
    
    fn handle_end_tag(&mut self, buffer: &TextBuffer, iter: &mut TextIter, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Heading(_) => {
                buffer.insert(iter, "\n");
                if !self.format_stack.is_empty() {
                    self.format_stack.pop();
                }
            }
            TagEnd::Paragraph => {
                // Paragraph end handled by next element start
            }
            TagEnd::Emphasis | TagEnd::Strong => {
                if !self.format_stack.is_empty() {
                    self.format_stack.pop();
                }
            }
            TagEnd::Link => {
                if let Some((start_mark, url)) = self.pending_link.take() {
                    let end_mark = buffer.create_mark(None, iter, true); // left-gravity: stays at end of link text
                    self.link_ranges.push((start_mark, end_mark, url));
                }
                if !self.format_stack.is_empty() {
                    self.format_stack.pop();
                }
            }
            TagEnd::CodeBlock | TagEnd::BlockQuote(_) => {
                buffer.insert(iter, "\n");
                if !self.format_stack.is_empty() {
                    self.format_stack.pop();
                }
            }
            TagEnd::Item => {
                buffer.insert(iter, "\n");
            }
            TagEnd::TableCell => {
                self.in_table_cell = false;
                self.current_row.push((self.current_cell_text.clone(), self.in_table_head));
            }
            TagEnd::TableRow => {
                self.table_data.push(self.current_row.clone());
                self.current_row.clear();
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::Table => {
                self.in_table = false;
                self.render_table_widget(buffer, iter);
            }
            _ => {}
        }
    }
    
    fn render_table_widget(&self, buffer: &TextBuffer, iter: &mut TextIter) {
        let Some(ref tv) = self.text_view else { return };
        if self.table_data.is_empty() { return; }

        let anchor = buffer.create_child_anchor(iter);
        buffer.insert(iter, "\n\n");

        let frame = gtk4::Frame::new(None);
        frame.add_css_class("md-table-frame");
        let grid = gtk4::Grid::new();
        grid.add_css_class("md-table");
        frame.set_child(Some(&grid));

        for (row_idx, row) in self.table_data.iter().enumerate() {
            for (col_idx, (text, is_header)) in row.iter().enumerate() {
                let label = gtk4::Label::new(None);
                let xalign = match self.table_alignments.get(col_idx) {
                    Some(Alignment::Right)  => 1.0f32,
                    Some(Alignment::Center) => 0.5f32,
                    _ => 0.0f32,
                };
                label.set_xalign(xalign);
                label.set_margin_start(8);
                label.set_margin_end(8);
                label.set_margin_top(4);
                label.set_margin_bottom(4);
                label.set_wrap(true);
                if *is_header {
                    label.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(text)));
                    label.add_css_class("md-table-header");
                } else {
                    label.set_text(text);
                    label.add_css_class("md-table-cell");
                }
                grid.attach(&label, col_idx as i32, row_idx as i32, 1, 1);
            }
        }

        tv.add_child_at_anchor(&frame, &anchor);
    }

    fn insert_text(&self, buffer: &TextBuffer, iter: &mut TextIter, text: &str) {
        if self.format_stack.is_empty() {
            buffer.insert(iter, text);
        } else {
            // Apply all active formatting tags
            let tags: Vec<&TextTag> = self.format_stack.iter().collect();
            buffer.insert_with_tags(iter, text, &tags);
        }
    }
}

/// A parsed segment from streaming think-tag processing.
///
/// Separating the pure detection logic (see [`parse_think_segments`]) from GTK mutations lets us
/// unit-test the state machine without a live display session.
#[derive(Debug, PartialEq)]
enum StreamSegment {
    /// Plain text that passes through the markdown renderer.
    Normal(String),
    /// The `<think>` opening boundary was seen; the GTK layer emits an indicator.
    ThinkStart,
    /// Content inside a think block, rendered with the think-tag style.
    Think(String),
    /// The `</think>` closing boundary was seen; the GTK layer emits a separator.
    ThinkEnd,
}

/// Parse `text` into typed [`StreamSegment`]s, updating the in-flight `in_think` cursor.
///
/// Streaming-safe: a think block may open in one call and close in a later call.
fn parse_think_segments(text: &str, in_think: &mut bool) -> Vec<StreamSegment> {
    let mut segments = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if *in_think {
            match remaining.find("</think>") {
                Some(end_pos) => {
                    let content = &remaining[..end_pos];
                    if !content.is_empty() {
                        segments.push(StreamSegment::Think(content.to_string()));
                    }
                    segments.push(StreamSegment::ThinkEnd);
                    *in_think = false;
                    remaining = &remaining[end_pos + 8..]; // skip "</think>"
                }
                None => {
                    segments.push(StreamSegment::Think(remaining.to_string()));
                    break;
                }
            }
        } else {
            match remaining.find("<think>") {
                Some(start_pos) => {
                    let before = &remaining[..start_pos];
                    if !before.is_empty() {
                        segments.push(StreamSegment::Normal(before.to_string()));
                    }
                    segments.push(StreamSegment::ThinkStart);
                    *in_think = true;
                    remaining = &remaining[start_pos + 7..]; // skip "<think>"
                }
                None => {
                    segments.push(StreamSegment::Normal(remaining.to_string()));
                    break;
                }
            }
        }
    }

    segments
}

/// Helper function to parse color strings (hex format) into RGBA
fn parse_color(color_str: &str) -> Result<gtk4::gdk::RGBA, Box<dyn std::error::Error>> {
    let color_str = color_str.trim_start_matches('#');
    
    if color_str.len() != 6 {
        return Err("Color must be in #RRGGBB format".into());
    }
    
    let r = u8::from_str_radix(&color_str[0..2], 16)? as f32 / 255.0;
    let g = u8::from_str_radix(&color_str[2..4], 16)? as f32 / 255.0;
    let b = u8::from_str_radix(&color_str[4..6], 16)? as f32 / 255.0;
    
    Ok(gtk4::gdk::RGBA::new(r, g, b, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_color ──────────────────────────────────────────────────────────

    #[test]
    fn parse_color_red() {
        let c = parse_color("#ff0000").unwrap();
        assert!((c.red() - 1.0).abs() < 1e-4);
        assert!((c.green() - 0.0).abs() < 1e-4);
        assert!((c.blue() - 0.0).abs() < 1e-4);
        assert!((c.alpha() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn parse_color_black() {
        let c = parse_color("#000000").unwrap();
        assert!((c.red() - 0.0).abs() < 1e-4);
        assert!((c.green() - 0.0).abs() < 1e-4);
        assert!((c.blue() - 0.0).abs() < 1e-4);
    }

    #[test]
    fn parse_color_white() {
        let c = parse_color("#ffffff").unwrap();
        assert!((c.red() - 1.0).abs() < 1e-4);
        assert!((c.green() - 1.0).abs() < 1e-4);
        assert!((c.blue() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn parse_color_short_hex_is_error() {
        assert!(parse_color("#fff").is_err());
    }

    #[test]
    fn parse_color_non_hex_chars_is_error() {
        assert!(parse_color("#zzzzzz").is_err());
    }

    #[test]
    fn parse_color_empty_is_error() {
        assert!(parse_color("").is_err());
    }

    // ── parse_think_segments ─────────────────────────────────────────────────

    #[test]
    fn plain_text_produces_single_normal_segment() {
        let mut in_think = false;
        let segs = parse_think_segments("hello world", &mut in_think);
        assert_eq!(segs, vec![StreamSegment::Normal("hello world".into())]);
        assert!(!in_think);
    }

    #[test]
    fn think_block_in_middle_produces_all_segments() {
        let mut in_think = false;
        let segs = parse_think_segments("before <think>thinking</think> after", &mut in_think);
        assert_eq!(segs, vec![
            StreamSegment::Normal("before ".into()),
            StreamSegment::ThinkStart,
            StreamSegment::Think("thinking".into()),
            StreamSegment::ThinkEnd,
            StreamSegment::Normal(" after".into()),
        ]);
        assert!(!in_think);
    }

    #[test]
    fn unclosed_think_tag_leaves_in_think_true() {
        let mut in_think = false;
        let segs = parse_think_segments("start <think>partial", &mut in_think);
        assert_eq!(segs, vec![
            StreamSegment::Normal("start ".into()),
            StreamSegment::ThinkStart,
            StreamSegment::Think("partial".into()),
        ]);
        assert!(in_think);
    }

    #[test]
    fn closing_tag_in_second_call_closes_correctly() {
        let mut in_think = true; // simulate carrying over from previous call
        let segs = parse_think_segments("rest</think> normal", &mut in_think);
        assert_eq!(segs, vec![
            StreamSegment::Think("rest".into()),
            StreamSegment::ThinkEnd,
            StreamSegment::Normal(" normal".into()),
        ]);
        assert!(!in_think);
    }

    #[test]
    fn continuation_while_in_think_produces_think_segment() {
        let mut in_think = true;
        let segs = parse_think_segments("more thinking...", &mut in_think);
        assert_eq!(segs, vec![StreamSegment::Think("more thinking...".into())]);
        assert!(in_think);
    }

    #[test]
    fn empty_think_block_produces_start_and_end_only() {
        let mut in_think = false;
        let segs = parse_think_segments("<think></think>", &mut in_think);
        assert_eq!(segs, vec![
            StreamSegment::ThinkStart,
            StreamSegment::ThinkEnd,
        ]);
        assert!(!in_think);
    }

    #[test]
    fn think_block_at_very_start() {
        let mut in_think = false;
        let segs = parse_think_segments("<think>reasoning</think>answer", &mut in_think);
        assert_eq!(segs, vec![
            StreamSegment::ThinkStart,
            StreamSegment::Think("reasoning".into()),
            StreamSegment::ThinkEnd,
            StreamSegment::Normal("answer".into()),
        ]);
        assert!(!in_think);
    }
}