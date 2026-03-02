use gtk::prelude::*;
use gtk::{Align, glib};
use std::sync::mpsc;
use std::time::Duration;

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

    let (unlock_result_tx, unlock_result_rx) = mpsc::channel::<crate::ipc::UnlockResult>();

    let result_label = message.clone();
    let result_entry = password_entry.clone();
    let result_window = window.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        while let Ok(result) = unlock_result_rx.try_recv() {
            result_entry.set_sensitive(true);

            match result {
                crate::ipc::UnlockResult::Accepted => {
                    result_label.remove_css_class("error");
                    result_label.remove_css_class("pending");
                    result_label.add_css_class("ok");
                    result_label.set_text("Unlock accepted by daemon.");
                    result_window.close();
                }
                crate::ipc::UnlockResult::Rejected => {
                    result_label.remove_css_class("ok");
                    result_label.remove_css_class("pending");
                    result_label.add_css_class("error");
                    result_label.set_text("Wrong password.");
                }
                crate::ipc::UnlockResult::TransportError(err) => {
                    result_label.remove_css_class("ok");
                    result_label.remove_css_class("pending");
                    result_label.add_css_class("error");
                    result_label.set_text(&format!("IPC error: {err}"));
                }
            }

            result_label.set_visible(true);
        }

        glib::ControlFlow::Continue
    });

    let submit_tx = unlock_result_tx.clone();
    let submit_label = message.clone();
    password_entry.connect_activate(move |entry| {
        let password = entry.text().to_string();
        if password.is_empty() {
            submit_label.remove_css_class("ok");
            submit_label.remove_css_class("pending");
            submit_label.add_css_class("error");
            submit_label.set_text("Password cannot be empty.");
            submit_label.set_visible(true);
            return;
        }

        submit_label.remove_css_class("ok");
        submit_label.remove_css_class("error");
        submit_label.add_css_class("pending");
        submit_label.set_text("Checking password with daemon...");
        submit_label.set_visible(true);

        entry.set_sensitive(false);
        entry.set_text("");

        let tx = submit_tx.clone();
        std::thread::spawn(move || {
            let result = crate::ipc::unlock_attempt(password);
            let _ = tx.send(result);
        });
    });

    window.present();
    password_entry.grab_focus();
}
