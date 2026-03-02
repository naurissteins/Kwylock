use gtk::prelude::*;
use gtk::{Align, glib};
use std::sync::mpsc;
use std::time::Duration;

pub fn build_window(app: &gtk::Application) {
    crate::style::install();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Kwylock")
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

    let subtitle = gtk::Label::new(Some("Wayland / Hyprland"));
    subtitle.add_css_class("subtitle");

    let status_line = gtk::Label::new(Some("Enter password to unlock."));
    status_line.add_css_class("status-line");
    status_line.add_css_class("status-info");

    let password_entry = gtk::Entry::builder()
        .placeholder_text("Password")
        .visibility(false)
        .max_width_chars(24)
        .build();
    password_entry.set_input_purpose(gtk::InputPurpose::Password);
    password_entry.add_css_class("password");

    root.append(&title);
    root.append(&time_label);
    root.append(&subtitle);
    root.append(&status_line);
    root.append(&password_entry);

    window.set_child(Some(&root));

    let clock_label = time_label.clone();
    glib::timeout_add_seconds_local(1, move || {
        clock_label.set_text(&crate::state::time_text());
        glib::ControlFlow::Continue
    });

    let (unlock_result_tx, unlock_result_rx) = mpsc::channel::<crate::ipc::UnlockResult>();

    let result_status = status_line.clone();
    let result_entry = password_entry.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        while let Ok(result) = unlock_result_rx.try_recv() {
            result_entry.set_sensitive(true);

            match result {
                crate::ipc::UnlockResult::Accepted => {
                    result_status.remove_css_class("status-warn");
                    result_status.remove_css_class("status-info");
                    result_status.add_css_class("status-ok");
                    result_status.set_text("Password accepted. Waiting for session unlock...");
                    result_entry.set_sensitive(false);
                }
                crate::ipc::UnlockResult::Rejected => {
                    result_status.remove_css_class("status-ok");
                    result_status.remove_css_class("status-info");
                    result_status.add_css_class("status-warn");
                    result_status.set_text("Invalid password.");
                }
                crate::ipc::UnlockResult::Failed(reason) => {
                    result_status.remove_css_class("status-ok");
                    result_status.remove_css_class("status-info");
                    result_status.add_css_class("status-warn");
                    result_status.set_text(&reason);
                }
                crate::ipc::UnlockResult::TransportError(_err) => {
                    result_status.remove_css_class("status-ok");
                    result_status.remove_css_class("status-info");
                    result_status.add_css_class("status-warn");
                    result_status.set_text("Cannot reach daemon. Check daemon logs.");
                }
            }
        }

        glib::ControlFlow::Continue
    });

    let submit_tx = unlock_result_tx.clone();
    let submit_status = status_line.clone();
    password_entry.connect_activate(move |entry| {
        let password = entry.text().to_string();
        if password.is_empty() {
            submit_status.remove_css_class("status-ok");
            submit_status.remove_css_class("status-info");
            submit_status.add_css_class("status-warn");
            submit_status.set_text("Password is required.");
            return;
        }

        submit_status.remove_css_class("status-ok");
        submit_status.remove_css_class("status-warn");
        submit_status.add_css_class("status-info");
        submit_status.set_text("Checking credentials...");

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
