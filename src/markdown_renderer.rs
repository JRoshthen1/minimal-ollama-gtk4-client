use gtk4::prelude::*;
use gtk4::{TextBuffer, TextTag, TextIter};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, Options};
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
        }
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
    
    /// Process text for streaming, handling think tags in real-time
    fn process_streaming_text(&mut self, buffer: &TextBuffer, text: &str, iter: &mut TextIter) -> String {
        let mut result = String::new();
        let mut remaining = text;
        
        while !remaining.is_empty() {
            if self.in_think_tag {
                // We're currently inside a think tag, look for closing tag
                if let Some(end_pos) = remaining.find("</think>") {
                    // Found closing tag - stream the remaining think content
                    let final_think_content = &remaining[..end_pos];
                    if !final_think_content.is_empty() {
                        buffer.insert_with_tags(iter, final_think_content, &[&self.think_tag]);
                    }
                    
                    // Close the think section
                    buffer.insert(iter, "\n\n");
                    
                    // Reset think state
                    self.in_think_tag = false;
                    
                    // Continue with text after closing tag
                    remaining = &remaining[end_pos + 8..]; // 8 = "</think>".len()
                } else {
                    // No closing tag yet, stream the think content as it arrives
                    if !remaining.is_empty() {
                        buffer.insert_with_tags(iter, remaining, &[&self.think_tag]);
                    }
                    break; // Wait for more streaming content
                }
            } else {
                // Not in think tag, look for opening tag
                if let Some(start_pos) = remaining.find("<think>") {
                    // Add content before think tag to result for normal processing
                    result.push_str(&remaining[..start_pos]);
                    
                    // Start think mode and show the think indicator
                    self.in_think_tag = true;
                    buffer.insert(iter, "\n💭 ");
                    
                    // Continue with content after opening tag
                    remaining = &remaining[start_pos + 7..]; // 7 = "<think>".len()
                } else {
                    // No think tag found, add all remaining text to result
                    result.push_str(remaining);
                    break;
                }
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
                self.insert_text(buffer, iter, &text);
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
            Tag::Link { .. } => Some(&self.link_tag),
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
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
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
            _ => {}
        }
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