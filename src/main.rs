use gtk4::prelude::*;
use gtk4::{glib, Application};

mod app;
mod state;
mod api;
mod types;
mod markdown_renderer;
mod ui;
mod config;
mod db;

const APP_ID: &str = "com.example.ollama-chat-gtk4";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(app::build_ui);
    app.run()
}