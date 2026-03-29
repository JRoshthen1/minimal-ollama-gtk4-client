use gtk4::prelude::*;
use gtk4::glib::clone;
use gtk4::{
    Box as GtkBox, Button, MenuButton, Orientation, DropDown, Label, StringList,
    Separator, ListBox, ListBoxRow, Popover, ScrolledWindow, PolicyType, SelectionMode,
    Image,
};

#[derive(Clone)]
pub struct ControlsArea {
    pub container: GtkBox,
    // Hidden DropDowns kept for signal/state compatibility with handlers.rs
    pub model_dropdown: DropDown,
    pub profile_dropdown: DropDown,
    pub status_label: Label,
    pub settings_button: Button,
    pub thinking_button: Button,
    models: StringList,
    profiles: StringList,
    model_button_label: Label,
    profile_button_label: Label,
    model_list: ListBox,
    profile_list: ListBox,
}

impl ControlsArea {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 2);
        container.add_css_class("toolbar");
        container.set_margin_top(4);
        container.set_margin_bottom(8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_hexpand(true);

        // --- Hidden model DropDown (state + signals only) ---
        let models = StringList::new(&[]);
        let model_dropdown = DropDown::new(Some(models.clone()), None::<gtk4::Expression>);
        model_dropdown.set_visible(false);

        // --- Model button label (shows currently selected model inline) ---
        let model_button_label = Label::new(Some("—"));
        model_button_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        model_button_label.set_max_width_chars(22);
        model_button_label.set_xalign(0.0);

        // Model button: icon + label in an HBox
        let model_btn_content = GtkBox::new(Orientation::Horizontal, 6);
        model_btn_content.set_margin_start(2);
        model_btn_content.set_margin_end(2);
        model_btn_content.append(&Image::from_icon_name("computer-symbolic"));
        model_btn_content.append(&model_button_label);

        let model_list = ListBox::new();
        model_list.set_selection_mode(SelectionMode::None);
        model_list.add_css_class("selector-list");

        let model_scroll = ScrolledWindow::new();
        model_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        model_scroll.set_max_content_height(300);
        model_scroll.set_propagate_natural_height(true);
        model_scroll.set_min_content_width(220);
        model_scroll.set_child(Some(&model_list));

        let model_popover = Popover::new();
        model_popover.set_child(Some(&model_scroll));

        let model_button = MenuButton::new();
        model_button.set_child(Some(&model_btn_content));
        model_button.set_popover(Some(&model_popover));
        model_button.set_always_show_arrow(false);
        model_button.add_css_class("toolbar-button");
        model_button.set_tooltip_text(Some("Select model"));

        // Sync hidden dropdown selection → button label (e.g. when profile overrides model)
        model_dropdown.connect_selected_notify(clone!(
            #[strong] model_button_label,
            #[strong] models,
            move |dd| {
                let sel = dd.selected();
                let text = models.string(sel)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "—".to_string());
                model_button_label.set_text(&text);
            }
        ));

        // List row activated → update hidden dropdown + close popover
        model_list.connect_row_activated(clone!(
            #[strong] model_dropdown,
            #[strong] model_popover,
            move |_, row| {
                let idx = row.index();
                if idx >= 0 {
                    model_dropdown.set_selected(idx as u32);
                }
                model_popover.popdown();
            }
        ));

        // --- Hidden profile DropDown ---
        let profiles = StringList::new(&["None"]);
        let profile_dropdown = DropDown::new(Some(profiles.clone()), None::<gtk4::Expression>);
        profile_dropdown.set_visible(false);

        // --- Profile button label (shows currently active profile inline) ---
        let profile_button_label = Label::new(Some("None"));
        profile_button_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        profile_button_label.set_max_width_chars(16);
        profile_button_label.set_xalign(0.0);

        // Profile button: icon + label in an HBox
        let profile_btn_content = GtkBox::new(Orientation::Horizontal, 6);
        profile_btn_content.set_margin_start(2);
        profile_btn_content.set_margin_end(2);
        profile_btn_content.append(&Image::from_icon_name("avatar-default-symbolic"));
        profile_btn_content.append(&profile_button_label);

        let profile_list = ListBox::new();
        profile_list.set_selection_mode(SelectionMode::None);
        profile_list.add_css_class("selector-list");

        let profile_scroll = ScrolledWindow::new();
        profile_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        profile_scroll.set_max_content_height(300);
        profile_scroll.set_propagate_natural_height(true);
        profile_scroll.set_min_content_width(220);
        profile_scroll.set_child(Some(&profile_list));

        let profile_popover = Popover::new();
        profile_popover.set_child(Some(&profile_scroll));

        let profile_button = MenuButton::new();
        profile_button.set_child(Some(&profile_btn_content));
        profile_button.set_popover(Some(&profile_popover));
        profile_button.set_always_show_arrow(false);
        profile_button.add_css_class("toolbar-button");
        profile_button.set_tooltip_text(Some("Select profile"));

        profile_dropdown.connect_selected_notify(clone!(
            #[strong] profile_button_label,
            #[strong] profiles,
            move |dd| {
                let sel = dd.selected();
                let text = profiles.string(sel)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "None".to_string());
                profile_button_label.set_text(&text);
            }
        ));

        profile_list.connect_row_activated(clone!(
            #[strong] profile_dropdown,
            #[strong] profile_popover,
            move |_, row| {
                let idx = row.index();
                if idx >= 0 {
                    profile_dropdown.set_selected(idx as u32);
                }
                profile_popover.popdown();
            }
        ));

        // --- Thinking toggle ---
        let thinking_button = Button::from_icon_name("application-x-appliance-symbolic");
        thinking_button.set_tooltip_text(Some("Toggle thinking mode"));
        thinking_button.add_css_class("toolbar-button");

        // --- Flexible spacer pushes status + settings to the right ---
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        // --- Status indicator ---
        let status_label = Label::new(Some("●"));
        status_label.set_halign(gtk4::Align::Center);
        status_label.add_css_class("status-label");
        status_label.set_tooltip_text(Some("Ready"));
        status_label.set_margin_start(8);
        status_label.set_margin_end(4);

        // --- Vertical separator before settings ---
        let sep = Separator::new(Orientation::Vertical);
        sep.set_margin_start(4);
        sep.set_margin_end(4);

        // --- Settings button ---
        let settings_button = Button::from_icon_name("preferences-system-symbolic");
        settings_button.set_tooltip_text(Some("Settings"));
        settings_button.add_css_class("toolbar-button");

        // Hidden dropdowns don't need to be in the widget tree for signals to work,
        // but add them invisible so they stay alive and realized.
        container.append(&model_dropdown);
        container.append(&profile_dropdown);
        container.append(&model_button);
        container.append(&profile_button);
        container.append(&thinking_button);
        container.append(&spacer);
        container.append(&status_label);
        container.append(&sep);
        container.append(&settings_button);

        Self {
            container,
            model_dropdown,
            profile_dropdown,
            status_label,
            settings_button,
            thinking_button,
            models,
            profiles,
            model_button_label,
            profile_button_label,
            model_list,
            profile_list,
        }
    }

    pub fn set_models(&self, model_names: &[impl AsRef<str>]) {
        let model_names_refs: Vec<&str> = model_names.iter().map(|s| s.as_ref()).collect();
        self.models.splice(0, self.models.n_items(), &model_names_refs);

        while let Some(child) = self.model_list.first_child() {
            self.model_list.remove(&child);
        }
        for name in model_names {
            self.model_list.append(&make_list_row(name.as_ref()));
        }

        if !model_names.is_empty() {
            self.model_dropdown.set_selected(0); // fires notify → updates button label
        }
    }

    pub fn get_selected_model(&self) -> Option<String> {
        let selected = self.model_dropdown.selected();
        if selected != gtk4::INVALID_LIST_POSITION {
            self.models.string(selected).map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn set_profiles(&self, profile_names: &[String]) {
        let mut entries: Vec<&str> = vec!["None"];
        let name_refs: Vec<&str> = profile_names.iter().map(|s| s.as_str()).collect();
        entries.extend_from_slice(&name_refs);
        self.profiles.splice(0, self.profiles.n_items(), &entries);

        while let Some(child) = self.profile_list.first_child() {
            self.profile_list.remove(&child);
        }
        for name in &entries {
            self.profile_list.append(&make_list_row(name));
        }

        self.profile_dropdown.set_selected(0);
    }

    pub fn get_selected_profile_name(&self) -> Option<String> {
        let selected = self.profile_dropdown.selected();
        if selected == gtk4::INVALID_LIST_POSITION || selected == 0 {
            None
        } else {
            self.profiles.string(selected).map(|s| s.to_string())
        }
    }

    /// Update the status indicator. The dot colour reflects state; full text is in the tooltip.
    pub fn set_status(&self, status: &str) {
        self.status_label.set_tooltip_text(Some(status));
        self.status_label.remove_css_class("status-error");
        self.status_label.remove_css_class("status-busy");
        if status.starts_with("Error") || status.starts_with("error") {
            self.status_label.add_css_class("status-error");
        } else if status.contains("typing") || status.contains("Loading") || status.contains("loading") {
            self.status_label.add_css_class("status-busy");
        }
    }
}

fn make_list_row(text: &str) -> ListBoxRow {
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_margin_top(5);
    label.set_margin_bottom(5);
    label.set_margin_start(10);
    label.set_margin_end(10);
    let row = ListBoxRow::new();
    row.set_child(Some(&label));
    row
}

pub fn create_controls() -> ControlsArea {
    ControlsArea::new()
}
