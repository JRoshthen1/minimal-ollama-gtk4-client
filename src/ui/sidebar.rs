use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation,
    ScrolledWindow, SelectionMode, Separator,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::db::ConversationSummary;

/// Left-panel conversation list.
#[derive(Clone)]
pub struct ConversationSidebar {
    pub container: GtkBox,
    pub list_box: ListBox,
    pub new_button: Button,
    pub clear_all_button: Button,
    /// Conversation ids in the same order as the ListBox rows.
    ids: Rc<RefCell<Vec<i64>>>,
    /// Called with the conversation id when a row's delete button is clicked.
    on_delete: Rc<RefCell<Option<Box<dyn Fn(i64)>>>>,
}

impl ConversationSidebar {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_size_request(150, -1);
        container.add_css_class("sidebar");

        // Top button row: "New Chat" on the left, "Clear All" on the right
        let button_row = GtkBox::new(Orientation::Horizontal, 4);
        button_row.set_margin_top(8);
        button_row.set_margin_bottom(8);
        button_row.set_margin_start(8);
        button_row.set_margin_end(8);

        let new_button = Button::with_label("+ New");
        new_button.add_css_class("toolbar-button");
        new_button.set_hexpand(true);

        let clear_all_button = Button::with_label("Clear All");
        clear_all_button.add_css_class("toolbar-button");
        clear_all_button.add_css_class("destructive-button");

        button_row.append(&new_button);
        button_row.append(&clear_all_button);
        container.append(&button_row);

        // Thin separator under the buttons
        let sep = Separator::new(Orientation::Horizontal);
        container.append(&sep);

        // Scrollable conversation list
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Single);
        list_box.add_css_class("sidebar-list");
        scroll.set_child(Some(&list_box));
        container.append(&scroll);

        let ids = Rc::new(RefCell::new(Vec::<i64>::new()));
        let on_delete: Rc<RefCell<Option<Box<dyn Fn(i64)>>>> = Rc::new(RefCell::new(None));

        Self { container, list_box, new_button, clear_all_button, ids, on_delete }
    }

    /// Register a callback that fires when a row's delete button is clicked.
    /// Must be called before `populate` for the callback to be wired into rows.
    pub fn set_on_delete(&self, f: impl Fn(i64) + 'static) {
        *self.on_delete.borrow_mut() = Some(Box::new(f));
    }

    /// Rebuild the list from a fresh slice of summaries.
    pub fn populate(&self, conversations: &[ConversationSummary]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.ids.borrow_mut().clear();

        for conv in conversations {
            let row = self.make_row(conv.id, &conv.title, &conv.updated_at);
            self.list_box.append(&row);
            self.ids.borrow_mut().push(conv.id);
        }
    }

    /// Return the conversation id of the currently selected row, if any.
    pub fn selected_id(&self) -> Option<i64> {
        let row = self.list_box.selected_row()?;
        let index = row.index();
        if index < 0 {
            return None;
        }
        self.ids.borrow().get(index as usize).copied()
    }

    /// Highlight the row that corresponds to `conv_id`, if present.
    pub fn select_by_id(&self, conv_id: i64) {
        let ids = self.ids.borrow();
        if let Some(pos) = ids.iter().position(|&id| id == conv_id) {
            if let Some(row) = self.list_box.row_at_index(pos as i32) {
                self.list_box.select_row(Some(&row));
            }
        }
    }

    /// Deselect every row (used when starting a new chat).
    pub fn deselect(&self) {
        self.list_box.unselect_all();
    }

    fn make_row(&self, conv_id: i64, title: &str, updated_at: &str) -> ListBoxRow {
        // Horizontal layout: [title+date (expands)] [× delete button]
        let hbox = GtkBox::new(Orientation::Horizontal, 4);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(10);
        hbox.set_margin_end(6);

        let vbox = GtkBox::new(Orientation::Vertical, 2);
        vbox.set_hexpand(true);

        let title_label = Label::new(Some(title));
        title_label.set_halign(Align::Start);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(22);

        let date_label = Label::new(Some(&format_date(updated_at)));
        date_label.set_halign(Align::Start);
        date_label.add_css_class("dim-label");

        vbox.append(&title_label);
        vbox.append(&date_label);

        let delete_btn = Button::with_label("×");
        delete_btn.add_css_class("sidebar-delete-button");
        delete_btn.set_valign(Align::Center);
        delete_btn.set_tooltip_text(Some("Delete conversation"));

        // Wire delete callback
        let on_delete = self.on_delete.clone();
        delete_btn.connect_clicked(move |_| {
            if let Some(ref cb) = *on_delete.borrow() {
                cb(conv_id);
            }
        });

        hbox.append(&vbox);
        hbox.append(&delete_btn);

        let row = ListBoxRow::new();
        row.set_child(Some(&hbox));
        row
    }
}

pub fn create_sidebar() -> ConversationSidebar {
    ConversationSidebar::new()
}

/// Show only the date part of a SQLite datetime string (first 10 chars).
fn format_date(updated_at: &str) -> String {
    updated_at.get(..10).unwrap_or(updated_at).to_string()
}
