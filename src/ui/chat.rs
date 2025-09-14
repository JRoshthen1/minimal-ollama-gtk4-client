use gtk4::prelude::*;
use gtk4::{TextView, TextBuffer, ScrolledWindow, WrapMode, PolicyType, Box as GtkBox, Orientation};
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
        
        // Add spacing if buffer is not empty
        if self.text_buffer.char_count() > 0 {
            self.text_buffer.insert(&mut end_iter, "\n\n");
            end_iter = self.text_buffer.end_iter();
        }
        
        // Add sender label with bold formatting
        let sender_tag = gtk4::TextTag::new(Some("sender"));
        sender_tag.set_weight(700);
        sender_tag.set_property("pixels-below-lines", 4);
        
        // Add the sender tag to the buffer's tag table if it's not already there
        let tag_table = self.text_buffer.tag_table();
        if tag_table.lookup("sender").is_none() {
            tag_table.add(&sender_tag);
        }
        
        self.text_buffer.insert_with_tags(&mut end_iter, &format!("{}:\n", sender), &[&sender_tag]);
        end_iter = self.text_buffer.end_iter();
        
        // Add message - format markdown for assistant, plain text for user
        if sender == "You" {
            self.text_buffer.insert(&mut end_iter, message);
        } else {
            self.insert_formatted_text(message, &mut end_iter, config);
        }
    }
    
    pub fn insert_formatted_text(&self, markdown_text: &str, iter: &mut gtk4::TextIter, config: &Config) {
        let mut renderer = self.markdown_renderer.borrow_mut();
        renderer.render_markdown_at_iter(&self.text_buffer, markdown_text, iter, config);
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
        
        // Get a fresh iterator at the mark position after deletion
        let _insert_iter = self.text_buffer.iter_at_mark(mark);
        
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