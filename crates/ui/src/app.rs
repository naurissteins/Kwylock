use gtk::prelude::*;

const APP_ID: &str = "io.kwylock.lockscreen";

pub fn run() {
    let app = gtk::Application::builder().application_id(APP_ID).build();

    app.connect_activate(|app| {
        crate::ui::build_window(app);
    });

    let _exit = app.run();
}
