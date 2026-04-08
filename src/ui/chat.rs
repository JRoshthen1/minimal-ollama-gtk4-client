use gtk4::prelude::*;
use gtk4::{TextView, TextBuffer, ScrolledWindow, WrapMode, PolicyType, Box as GtkBox, Orientation};
use gtk4::gio;
use std::rc::Rc;
use std::cell::RefCell;
use crate::markdown_renderer::MarkdownRenderer;
use crate::config::Config;

#[derive(Clone)]
pub struct ChatView {
    scrolled_window: ScrolledWindow,
    text_view: TextView,
    text_buffer: TextBuffer,
    main_container: GtkBox,
    markdown_renderer: Rc<RefCell<MarkdownRenderer>>,
}

impl ChatView {
    pub fn new() -> Self {
        // Create main container that can hold both text and widgets
        let main_container = GtkBox::new(Orientation::Vertical, 8);
        
        let scrolled_window = ScrolledWindow::new();
        scrolled_window.add_css_class("chat-container");
        scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled_window.set_vexpand(true);  // Allow vertical expansion
        scrolled_window.set_hexpand(true);  // Allow horizontal expansion
        
        let text_view = TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(WrapMode::WordChar);
        text_view.set_hexpand(true);
        text_view.set_vexpand(true);
        text_view.add_css_class("chat-text");
        text_view.set_margin_start(12);
        text_view.set_margin_end(12);
        text_view.set_margin_top(12);
        text_view.set_margin_bottom(12);
        
        let text_buffer = TextBuffer::new(None);
        text_view.set_buffer(Some(&text_buffer));
        
        let markdown_renderer = Rc::new(RefCell::new(MarkdownRenderer::new()));
        markdown_renderer.borrow_mut().set_text_view(text_view.clone());

        // Clickable links: open URL on click
        let gesture = gtk4::GestureClick::new();
        let tv_click = text_view.clone();
        let buf_click = text_buffer.clone();
        let renderer_click = markdown_renderer.clone();
        gesture.connect_released(move |_, _n, x, y| {
            let (bx, by) = tv_click.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
            if let Some(iter) = tv_click.iter_at_location(bx, by) {
                let offset = iter.offset();
                let renderer = renderer_click.borrow();
                for (sm, em, url) in &renderer.link_ranges {
                    let start = buf_click.iter_at_mark(sm).offset();
                    let end = buf_click.iter_at_mark(em).offset();
                    if offset >= start && offset < end {
                        let _ = gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>);
                        break;
                    }
                }
            }
        });
        text_view.add_controller(gesture);

        // Pointer cursor when hovering over links
        let motion = gtk4::EventControllerMotion::new();
        let tv_motion = text_view.clone();
        let buf_motion = text_buffer.clone();
        let renderer_motion = markdown_renderer.clone();
        motion.connect_motion(move |_, x, y| {
            let (bx, by) = tv_motion.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
            let over_link = tv_motion.iter_at_location(bx, by).map_or(false, |iter| {
                let offset = iter.offset();
                let renderer = renderer_motion.borrow();
                renderer.link_ranges.iter().any(|(sm, em, _)| {
                    let start = buf_motion.iter_at_mark(sm).offset();
                    let end = buf_motion.iter_at_mark(em).offset();
                    offset >= start && offset < end
                })
            });
            tv_motion.set_cursor_from_name(Some(if over_link { "pointer" } else { "text" }));
        });
        text_view.add_controller(motion);

        // Add text view to container
        main_container.append(&text_view);
        main_container.set_vexpand(true);
        main_container.set_hexpand(true);
        
        scrolled_window.set_child(Some(&main_container));
        
        Self {
            scrolled_window,
            text_view,
            text_buffer,
            main_container,
            markdown_renderer,
        }
    }
    
    pub fn widget(&self) -> &ScrolledWindow {
        &self.scrolled_window
    }
    
    pub fn buffer(&self) -> &TextBuffer {
        &self.text_buffer
    }
    
    pub fn append_message(&self, sender: &str, message: &str, config: &Config) {
        let mut end_iter = self.text_buffer.end_iter();

        if self.text_buffer.char_count() > 0 {
            self.text_buffer.insert(&mut end_iter, "\n\n");
            end_iter = self.text_buffer.end_iter();
        }

        let is_user = sender == "You";
        let tag_table = self.text_buffer.tag_table();

        // Sender label tag — distinct per role
        let sender_tag_name = if is_user { "sender_user" } else { "sender_assistant" };
        let sender_tag = if let Some(existing) = tag_table.lookup(sender_tag_name) {
            existing
        } else {
            let tag = gtk4::TextTag::new(Some(sender_tag_name));
            tag.set_weight(700);
            tag.set_property("pixels-below-lines", 4);
            if is_user {
                tag.set_property("foreground", config.colors.link_text.as_str());
                tag.set_property("justification", gtk4::Justification::Right);
                tag.set_property("justification-set", true);
            }
            tag_table.add(&tag);
            tag
        };

        self.text_buffer.insert_with_tags(&mut end_iter, &format!("{}:\n", sender), &[&sender_tag]);
        end_iter = self.text_buffer.end_iter();

        if is_user {
            // Right-aligned user message with a left margin to push it away from the left edge
            let user_msg_tag = if let Some(existing) = tag_table.lookup("user_message") {
                existing
            } else {
                let tag = gtk4::TextTag::new(Some("user_message"));
                tag.set_property("justification", gtk4::Justification::Right);
                tag.set_property("justification-set", true);
                tag.set_property("left-margin", 80i32);
                tag.set_property("left-margin-set", true);
                tag_table.add(&tag);
                tag
            };
            self.text_buffer.insert_with_tags(&mut end_iter, message, &[&user_msg_tag]);
        } else {
            self.insert_formatted_text(message, &mut end_iter, config);
        }
    }

    /// Insert a styled "Assistant:" header and return a mark at the end for streaming content.
    pub fn begin_assistant_response(&self, config: &Config) -> gtk4::TextMark {
        let tag_table = self.text_buffer.tag_table();
        let mut end_iter = self.text_buffer.end_iter();

        if self.text_buffer.char_count() > 0 {
            self.text_buffer.insert(&mut end_iter, "\n\n");
            end_iter = self.text_buffer.end_iter();
        }

        let sender_tag = if let Some(existing) = tag_table.lookup("sender_assistant") {
            existing
        } else {
            let tag = gtk4::TextTag::new(Some("sender_assistant"));
            tag.set_weight(700);
            tag.set_property("pixels-below-lines", 4);
            tag_table.add(&tag);
            tag
        };

        // Silence unused warning — config is available for future use (e.g. per-role color)
        let _ = config;

        self.text_buffer.insert_with_tags(&mut end_iter, "Assistant:\n", &[&sender_tag]);
        end_iter = self.text_buffer.end_iter();
        self.text_buffer.create_mark(None, &end_iter, true)
    }
    
    pub fn insert_formatted_text(&self, markdown_text: &str, iter: &mut gtk4::TextIter, config: &Config) {
        let mut renderer = self.markdown_renderer.borrow_mut();
        renderer.render_markdown_at_iter(&self.text_buffer, markdown_text, iter, config);
    }
    
    /// Wipe all rendered content from the chat area (used when loading a different conversation).
    pub fn clear(&self) {
        self.text_buffer.delete(
            &mut self.text_buffer.start_iter(),
            &mut self.text_buffer.end_iter(),
        );
        self.markdown_renderer.borrow_mut().link_ranges.clear();
    }

    pub fn scroll_to_bottom(&self) {
        let adjustment = self.scrolled_window.vadjustment();
        adjustment.set_value(adjustment.upper() - adjustment.page_size());
    }
    
    pub fn create_mark_at_end(&self) -> gtk4::TextMark {
        self.text_buffer.create_mark(None, &self.text_buffer.end_iter(), true)
    }
    
    pub fn insert_formatted_at_mark(&self, mark: &gtk4::TextMark, content: &str, config: &Config) {
        let mut start_iter = self.text_buffer.iter_at_mark(mark);
        let mut end_iter = self.text_buffer.end_iter();
        
        self.text_buffer.delete(&mut start_iter, &mut end_iter);
        let mut insert_iter = self.text_buffer.iter_at_mark(mark);
        self.insert_formatted_text(content, &mut insert_iter, config);
    }
    
    pub fn update_streaming_markdown(&self, mark: &gtk4::TextMark, accumulated_content: &str, config: &Config) {
        // Store the current scroll position
        let adjustment = self.scrolled_window.vadjustment();
        let scroll_position = adjustment.value();
        let at_bottom = scroll_position >= (adjustment.upper() - adjustment.page_size() - 50.0);
        
        // Get the mark position
        let mut start_iter = self.text_buffer.iter_at_mark(mark);
        let mut end_iter = self.text_buffer.end_iter();
        
        // Delete existing content from mark to end
        self.text_buffer.delete(&mut start_iter, &mut end_iter);

        // Render markdown directly to the main buffer
        // We use a separate method to avoid conflicts with the borrow checker
        self.render_markdown_at_mark(mark, accumulated_content, config);
        
        // Restore scroll position or scroll to bottom if we were at the bottom
        if at_bottom {
            self.scroll_to_bottom();
        } else {
            adjustment.set_value(scroll_position);
        }
    }
    
    fn render_markdown_at_mark(&self, mark: &gtk4::TextMark, content: &str, config: &Config) {
        let mut insert_iter = self.text_buffer.iter_at_mark(mark);
        
        // Create a new scope to ensure the borrow is dropped
        {
            let mut renderer = self.markdown_renderer.borrow_mut();
            renderer.render_markdown_at_iter(&self.text_buffer, content, &mut insert_iter, config);
        }
    }
}

pub fn create_chat_view() -> ChatView {
    ChatView::new()
}