mod error;
mod state;
mod ui;

use ui::App;

fn main() {
    if gtk::init().is_err() {
        eprintln!("Failed to init GTK.");
        return;
    }

    let app = App::new();
    app.run();
}
