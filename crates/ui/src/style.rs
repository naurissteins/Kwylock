use gtk::{CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION, gdk};

pub fn install() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("../assets/style.css"));

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
