use gtk::prelude::*;
use gtk::{Align, glib};

pub fn build_window(app: &gtk::Application) {
    crate::style::install();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Kwylock Prototype")
        .build();

    window.set_widget_name("lock-window");
    window.set_decorated(false);
    window.fullscreen();

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .halign(Align::Center)
        .valign(Align::Center)
        .spacing(14)
        .build();

    let title = gtk::Label::new(Some("Kwylock"));
    title.add_css_class("title");

    let time_label = gtk::Label::new(Some(&crate::state::time_text()));
    time_label.add_css_class("time");

    let subtitle = gtk::Label::new(Some("Wayland/Hyprland GTK4 prototype"));
    subtitle.add_css_class("subtitle");

    let password_entry = gtk::Entry::builder()
        .placeholder_text("Password")
        .visibility(false)
        .max_width_chars(24)
        .build();
    password_entry.set_input_purpose(gtk::InputPurpose::Password);
    password_entry.add_css_class("password");

    let message = gtk::Label::new(None);
    message.add_css_class("message");
    message.set_visible(false);

    root.append(&title);
    root.append(&time_label);
    root.append(&subtitle);
    root.append(&password_entry);
    root.append(&message);

    window.set_child(Some(&root));

    let clock_label = time_label.clone();
    glib::timeout_add_seconds_local(1, move || {
        clock_label.set_text(&crate::state::time_text());
        glib::ControlFlow::Continue
    });

    let status_label = message.clone();
    password_entry.connect_activate(move |entry| {
        let is_valid = entry.text().as_str() == "test";

        if is_valid {
            status_label.remove_css_class("error");
            status_label.add_css_class("ok");
            status_label.set_text("Prototype unlock accepted.");
        } else {
            status_label.remove_css_class("ok");
            status_label.add_css_class("error");
            status_label.set_text("Wrong password (prototype mode).");
        }

        status_label.set_visible(true);
        entry.set_text("");
    });

    window.present();
    password_entry.grab_focus();
}
