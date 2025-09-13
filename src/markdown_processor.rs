use gtk4::prelude::*;
use gtk4::{TextBuffer, TextTag, TextTagTable};
use std::collections::HashMap;

pub struct MarkdownProcessor {
    h1_tag: TextTag,
    h2_tag: TextTag,
    h3_tag: TextTag,
    h4_tag: TextTag,
    h5_tag: TextTag,
    h6_tag: TextTag,
    bold_tag: TextTag,
    italic_tag: TextTag,
    code_tag: TextTag,
}

impl MarkdownProcessor {
    pub fn new() -> Self {
        // Heading tags
        let h1_tag = TextTag::new(Some("h1"));
        h1_tag.set_weight(700);
        h1_tag.set_line_height(1.4);
        h1_tag.set_scale(1.5);
        
        let h2_tag = TextTag::new(Some("h2"));
        h2_tag.set_weight(700);
        h2_tag.set_line_height(1.4);
        h2_tag.set_scale(1.4);
        
        let h3_tag = TextTag::new(Some("h3"));
        h3_tag.set_weight(700);
        h3_tag.set_line_height(1.2);
        h3_tag.set_scale(1.3);
        
        let h4_tag = TextTag::new(Some("h4"));
        h4_tag.set_weight(700);
        h4_tag.set_line_height(1.2);
        h4_tag.set_scale(1.2);
        
        let h5_tag = TextTag::new(Some("h5"));
        h5_tag.set_weight(700);
        h5_tag.set_line_height(1.2);
        h5_tag.set_scale(1.1);
        
        let h6_tag = TextTag::new(Some("h6"));
        h6_tag.set_weight(700);
        h6_tag.set_line_height(1.2);
        h6_tag.set_scale(1.0);
        
        // Inline formatting tags
        let bold_tag = TextTag::new(Some("bold"));
        bold_tag.set_weight(700);
        
        let italic_tag = TextTag::new(Some("italic"));
        italic_tag.set_style(gtk4::pango::Style::Italic);
        
        let code_tag = TextTag::new(Some("code"));
        code_tag.set_property("font", &"monospace");
        code_tag.set_scale(0.9);
        
        Self {
            h1_tag,
            h2_tag,
            h3_tag,
            h4_tag,
            h5_tag,
            h6_tag,
            bold_tag,
            italic_tag,
            code_tag,
        }
    }
    
    pub fn setup_tags(&self, buffer: &TextBuffer) {
        let tag_table = buffer.tag_table();
        tag_table.add(&self.h1_tag);
        tag_table.add(&self.h2_tag);
        tag_table.add(&self.h3_tag);
        tag_table.add(&self.h4_tag);
        tag_table.add(&self.h5_tag);
        tag_table.add(&self.h6_tag);
        tag_table.add(&self.bold_tag);
        tag_table.add(&self.italic_tag);
        tag_table.add(&self.code_tag);
    }
    
    pub fn insert_formatted_text(&self, buffer: &TextBuffer, text: &str, iter: &mut gtk4::TextIter) {
        // Process the text line by line to handle headings and inline formatting
        let lines: Vec<&str> = text.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            // Check if this line is a heading
            if let Some(heading) = self.parse_heading(line) {
                let tag = match heading.level {
                    1 => &self.h1_tag,
                    2 => &self.h2_tag,
                    3 => &self.h3_tag,
                    4 => &self.h4_tag,
                    5 => &self.h5_tag,
                    _ => &self.h6_tag,
                };
                
                buffer.insert(iter, "\n");
                buffer.insert_with_tags(iter, &heading.content, &[tag]);
                buffer.insert(iter, "\n");
            } else {
                // Process inline formatting for non-heading lines
                self.insert_formatted_line(buffer, line, iter);
                
                // Add newline if not the last line
                if i < lines.len() - 1 {
                    buffer.insert(iter, "\n");
                }
            }
        }
    }
    
    fn insert_formatted_line(&self, buffer: &TextBuffer, line: &str, iter: &mut gtk4::TextIter) {
        let segments = self.parse_inline_formatting(line);
        
        for segment in segments {
            match segment.format_type {
                FormatType::Plain => {
                    buffer.insert(iter, &segment.content);
                }
                FormatType::Bold => {
                    buffer.insert_with_tags(iter, &segment.content, &[&self.bold_tag]);
                }
                FormatType::Italic => {
                    buffer.insert_with_tags(iter, &segment.content, &[&self.italic_tag]);
                }
                FormatType::Code => {
                    buffer.insert_with_tags(iter, &segment.content, &[&self.code_tag]);
                }
                FormatType::BoldItalic => {
                    buffer.insert_with_tags(iter, &segment.content, &[&self.bold_tag, &self.italic_tag]);
                }
                FormatType::Heading(_) => {
                    // This shouldn't happen in inline processing
                    buffer.insert(iter, &segment.content);
                }
            }
        }
    }
    
    fn parse_heading(&self, line: &str) -> Option<HeadingInfo> {
        if line.starts_with('#') {
            let hash_count = line.chars().take_while(|&c| c == '#').count();
            
            if hash_count <= 6 && line.len() > hash_count + 1 && line.chars().nth(hash_count) == Some(' ') {
                let content = &line[hash_count + 1..].trim();
                return Some(HeadingInfo {
                    level: hash_count,
                    content: content.to_string(),
                });
            }
        }
        None
    }
    
    fn parse_inline_formatting(&self, text: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let mut current_pos = 0;
        let chars: Vec<char> = text.chars().collect();
        
        while current_pos < chars.len() {
            // Look for the next formatting marker
            if let Some((marker_pos, marker_type, marker_len)) = self.find_next_marker(&chars, current_pos) {
                // Add any plain text before the marker
                if marker_pos > current_pos {
                    let plain_text: String = chars[current_pos..marker_pos].iter().collect();
                    if !plain_text.is_empty() {
                        segments.push(TextSegment {
                            content: plain_text,
                            format_type: FormatType::Plain,
                        });
                    }
                }
                
                // Find the closing marker
                if let Some((close_pos, close_len)) = self.find_closing_marker(&chars, marker_pos + marker_len, &marker_type) {
                    let content_start = marker_pos + marker_len;
                    let content_end = close_pos;
                    
                    if content_start < content_end {
                        let content: String = chars[content_start..content_end].iter().collect();
                        segments.push(TextSegment {
                            content,
                            format_type: marker_type,
                        });
                    }
                    
                    current_pos = close_pos + close_len;
                } else {
                    // No closing marker found, treat as plain text
                    let plain_char: String = chars[marker_pos..marker_pos + marker_len].iter().collect();
                    segments.push(TextSegment {
                        content: plain_char,
                        format_type: FormatType::Plain,
                    });
                    current_pos = marker_pos + marker_len;
                }
            } else {
                // No more markers, add the rest as plain text
                let remaining: String = chars[current_pos..].iter().collect();
                if !remaining.is_empty() {
                    segments.push(TextSegment {
                        content: remaining,
                        format_type: FormatType::Plain,
                    });
                }
                break;
            }
        }
        
        segments
    }
    
    fn find_next_marker(&self, chars: &[char], start_pos: usize) -> Option<(usize, FormatType, usize)> {
        let mut earliest_pos = None;
        let mut earliest_type = FormatType::Plain;
        let mut earliest_len = 0;
        
        for pos in start_pos..chars.len() {
            // Check for inline code (backticks)
            if chars[pos] == '`' {
                if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                    earliest_pos = Some(pos);
                    earliest_type = FormatType::Code;
                    earliest_len = 1;
                }
                break; // Prioritize code as it can contain other markers
            }
            
            // Check for bold/italic markers
            if pos + 1 < chars.len() {
                // Check for ** (bold) or *** (bold+italic)
                if chars[pos] == '*' && chars[pos + 1] == '*' {
                    if pos + 2 < chars.len() && chars[pos + 2] == '*' {
                        // *** bold+italic
                        if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                            earliest_pos = Some(pos);
                            earliest_type = FormatType::BoldItalic;
                            earliest_len = 3;
                        }
                    } else {
                        // ** bold
                        if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                            earliest_pos = Some(pos);
                            earliest_type = FormatType::Bold;
                            earliest_len = 2;
                        }
                    }
                    break;
                }
                
                // Check for __ (bold)
                if chars[pos] == '_' && chars[pos + 1] == '_' {
                    if earliest_pos.is_none() || pos < earliest_pos.unwrap() {
                        earliest_pos = Some(pos);
                        earliest_type = FormatType::Bold;
                        earliest_len = 2;
                    }
                    break;
                }
            }
            
            // Check for single * or _ (italic)
            if chars[pos] == '*' || chars[pos] == '_' {
                // Make sure it's not part of a double marker
                let is_single = (pos == 0 || chars[pos - 1] != chars[pos]) &&
                               (pos + 1 >= chars.len() || chars[pos + 1] != chars[pos]);
                
                if is_single && (earliest_pos.is_none() || pos < earliest_pos.unwrap()) {
                    earliest_pos = Some(pos);
                    earliest_type = FormatType::Italic;
                    earliest_len = 1;
                }
            }
        }
        
        earliest_pos.map(|pos| (pos, earliest_type, earliest_len))
    }
    
    fn find_closing_marker(&self, chars: &[char], start_pos: usize, marker_type: &FormatType) -> Option<(usize, usize)> {
        match marker_type {
            FormatType::Code => {
                // Look for closing backtick
                for pos in start_pos..chars.len() {
                    if chars[pos] == '`' {
                        return Some((pos, 1));
                    }
                }
            }
            FormatType::Bold => {
                // Look for closing ** or __
                for pos in start_pos..chars.len().saturating_sub(1) {
                    if (chars[pos] == '*' && chars[pos + 1] == '*') ||
                       (chars[pos] == '_' && chars[pos + 1] == '_') {
                        return Some((pos, 2));
                    }
                }
            }
            FormatType::Italic => {
                // Look for closing * or _
                for pos in start_pos..chars.len() {
                    if chars[pos] == '*' || chars[pos] == '_' {
                        // Make sure it's not part of a double marker
                        let is_single = (pos == 0 || chars[pos - 1] != chars[pos]) &&
                                       (pos + 1 >= chars.len() || chars[pos + 1] != chars[pos]);
                        if is_single {
                            return Some((pos, 1));
                        }
                    }
                }
            }
            FormatType::BoldItalic => {
                // Look for closing ***
                for pos in start_pos..chars.len().saturating_sub(2) {
                    if chars[pos] == '*' && chars[pos + 1] == '*' && chars[pos + 2] == '*' {
                        return Some((pos, 3));
                    }
                }
            }
            _ => {}
        }
        None
    }
}

#[derive(Debug, Clone)]
struct TextSegment {
    content: String,
    format_type: FormatType,
}

#[derive(Debug, Clone)]
struct HeadingInfo {
    level: usize,
    content: String,
}

#[derive(Debug, Clone, PartialEq)]
enum FormatType {
    Plain,
    Heading(usize),
    Bold,
    Italic,
    Code,
    BoldItalic,
}