use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, Orientation, Stack, TextView, Window, Align, ScrolledWindow};
use pango::WrapMode;
use std::cell::RefCell;
use std::rc::Rc;

use crate::state::{Row, State, Status};

const SCREEN_MAIN: &str = "main";
const SCREEN_TASKS: &str = "tasks";
const SCREEN_EDIT: &str = "edit";

pub struct App {
    pub window: Window,
    pub stack: Stack,
    pub state: Rc<RefCell<State>>,
    pub edit_context: Rc<RefCell<Option<EditContext>>>,
    pub edit_widgets: Rc<RefCell<Option<EditWidgets>>>,
}

pub struct EditContext {
    pub task_id: Option<u32>,
    pub date: String,
}

pub struct EditWidgets {
    pub header: Label,
    pub text_buffer: gtk::TextBuffer,
    pub status_combo: gtk::ComboBoxText,
    pub date_entry: Entry,
    pub delete_btn: Button,
}

struct Screens {
    date_store: gtk::ListStore,
    task_store: gtk::ListStore,
    date_label: Label,
}

impl App {
    pub fn new() -> Self {
        let window = Window::new(gtk::WindowType::Toplevel);
        window.set_title("Ежедневник");
        window.set_default_size(700, 550);
        window.set_position(gtk::WindowPosition::Center);

        let stack = Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

        // Load state from file or create new
        let state = Rc::new(RefCell::new(
            State::load().unwrap_or_else(|_| State::new())
        ));

        let edit_context = Rc::new(RefCell::new(None));
        let edit_widgets = Rc::new(RefCell::new(None));

        let app = Self { window, stack, state, edit_context, edit_widgets };
        app.setup_ui();
        app
    }

    fn setup_ui(&self) {
        let screens = Rc::new(RefCell::new(self.create_screens()));

        // Initial populate
        {
            let s = self.state.borrow();
            let dates = s.get_all_dates();
            let date_store = &screens.borrow().date_store;
            date_store.clear();
            for date in &dates {
                let iter = date_store.append();
                date_store.set_value(&iter, 0, &date.to_value());
            }
            
            let rows = s.get_rows_for_date(&s.cur_date);
            let task_store = &screens.borrow().task_store;
            task_store.clear();
            for row in rows {
                let iter = task_store.append();
                task_store.set(&iter, &[
                    (0, &row.id),
                    (1, &row.text),
                    (2, &row.status.to_str()),
                ]);
            }
            screens.borrow().date_label.set_markup(&format!("<span size='medium'>Дата: {}</span>", s.cur_date));
        }

        let main_screen = self.create_main_screen(&screens);
        let tasks_screen = self.create_tasks_screen(&screens);
        let edit_screen = self.create_edit_screen(&screens);

        self.stack.add_titled(&main_screen, SCREEN_MAIN, "Главная");
        self.stack.add_titled(&tasks_screen, SCREEN_TASKS, "Задачи");
        self.stack.add_titled(&edit_screen, SCREEN_EDIT, "Редактирование");

        // Refresh lists when switching screens
        let state_clone = self.state.clone();
        let screens_clone = screens.clone();
        let edit_context = self.edit_context.clone();
        let edit_widgets = self.edit_widgets.clone();
        self.stack.connect_visible_child_notify(move |stack| {
            let visible = stack.visible_child_name().unwrap_or_default();
            
            if visible == SCREEN_MAIN {
                // Refresh date list
                let dates = state_clone.borrow().get_all_dates();
                let date_store = &screens_clone.borrow().date_store;
                date_store.clear();
                for date in &dates {
                    let iter = date_store.append();
                    date_store.set_value(&iter, 0, &date.to_value());
                }
            } else if visible == SCREEN_TASKS {
                // Refresh task list
                let s = state_clone.borrow();
                let rows = s.get_rows_for_date(&s.cur_date);
                let task_store = &screens_clone.borrow().task_store;
                let date_label = &screens_clone.borrow().date_label;
                task_store.clear();
                for row in rows {
                    let iter = task_store.append();
                    task_store.set(&iter, &[
                        (0, &row.id),
                        (1, &row.text),
                        (2, &row.status.to_str()),
                    ]);
                }
                date_label.set_markup(&format!("<span size='medium'>Дата: {}</span>", s.cur_date));
            } else if visible == SCREEN_EDIT {
                // Handle edit screen - load task data if editing
                let mut ctx = edit_context.borrow_mut();
                if let Some(ref mut ctx) = *ctx {
                    // Loading existing task - set the date from current state
                    let s = state_clone.borrow();
                    ctx.date = s.cur_date.clone();
                    
                    // Populate widgets if we have a task to edit
                    if let Some(task_id) = ctx.task_id {
                        if let Some(row) = s.get_row(&ctx.date, task_id) {
                            if let Some(widgets) = edit_widgets.borrow().as_ref() {
                                widgets.header.set_markup("<span size='large' weight='bold'>✏️ Редактирование задачи</span>");
                                widgets.text_buffer.set_text(&row.text);
                                match row.status {
                                    Status::Working => widgets.status_combo.set_active(Some(0)),
                                    Status::Testing => widgets.status_combo.set_active(Some(1)),
                                    Status::Ready => widgets.status_combo.set_active(Some(2)),
                                }
                                widgets.date_entry.set_text(&ctx.date);
                                widgets.delete_btn.set_visible(true);
                            }
                        }
                    }
                } else {
                    // New task - clear widgets
                    if let Some(widgets) = edit_widgets.borrow().as_ref() {
                        widgets.header.set_markup("<span size='large' weight='bold'>✏️ Новая задача</span>");
                        widgets.text_buffer.set_text("");
                        widgets.status_combo.set_active(Some(0));
                        widgets.date_entry.set_text(&chrono::Local::now().format("%Y-%m-%d").to_string());
                        widgets.delete_btn.set_visible(false);
                    }
                }
            }
        });

        self.window.add(&self.stack);

        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            glib::Propagation::Proceed
        });
    }

    fn create_screens(&self) -> Screens {
        let date_store = gtk::ListStore::new(&[gtk::glib::Type::STRING]);
        let task_store = gtk::ListStore::new(&[
            gtk::glib::Type::U32,
            gtk::glib::Type::STRING,
            gtk::glib::Type::STRING,
        ]);
        let date_label = Label::new(None);

        Screens {
            date_store,
            task_store,
            date_label,
        }
    }

    fn create_main_screen(&self, screens: &Rc<RefCell<Screens>>) -> gtk::Widget {
        let vbox = GtkBox::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        let header = Label::new(None);
        header.set_markup("<span size='large' weight='bold'>📅 Выберите дату</span>");
        vbox.pack_start(&header, false, false, 5);

        // Date list using TreeView
        let scrolled = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_vexpand(true);

        let tree_view = gtk::TreeView::with_model(&screens.borrow().date_store);
        tree_view.set_headers_visible(false);

        let renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        gtk::prelude::CellLayoutExt::pack_start(&column, &renderer, true);
        gtk::prelude::CellLayoutExt::add_attribute(&column, &renderer, "text", 0);
        tree_view.append_column(&column);

        scrolled.add(&tree_view);
        vbox.pack_start(&scrolled, true, true, 5);

        // Buttons
        let today_btn = Button::with_label("Сегодня");
        today_btn.connect_clicked({
            let state = self.state.clone();
            let stack = self.stack.clone();
            move |_| {
                let mut s = state.borrow_mut();
                s.cur_date = chrono::Local::now().format("%Y-%m-%d").to_string();
                drop(s);
                stack.set_visible_child_name(SCREEN_TASKS);
            }
        });

        let new_task_btn = Button::with_label("+ Новая задача");
        new_task_btn.connect_clicked({
            let stack = self.stack.clone();
            move |_| {
                stack.set_visible_child_name(SCREEN_EDIT);
            }
        });

        let btn_box = GtkBox::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::Center);
        btn_box.pack_start(&today_btn, false, false, 5);
        btn_box.pack_start(&new_task_btn, false, false, 5);
        vbox.pack_start(&btn_box, false, false, 5);

        // Double-click to select date
        let state_clone = self.state.clone();
        let stack_clone = self.stack.clone();
        let tree_view_clone = tree_view.clone();
        tree_view.connect_row_activated(move |_, path, _| {
            let model = tree_view_clone.model().unwrap();
            let iter = model.iter(path).unwrap();
            let date: String = model.value(&iter, 0).get().unwrap();
            let mut s = state_clone.borrow_mut();
            s.cur_date = date;
            drop(s);
            stack_clone.set_visible_child_name(SCREEN_TASKS);
        });

        vbox.show_all();
        vbox.upcast()
    }

    fn create_tasks_screen(&self, screens: &Rc<RefCell<Screens>>) -> gtk::Widget {
        let vbox = GtkBox::new(Orientation::Vertical, 10);
        vbox.set_margin_top(10);
        vbox.set_margin_bottom(10);
        vbox.set_margin_start(10);
        vbox.set_margin_end(10);

        let header = Label::new(None);
        header.set_markup("<span size='large' weight='bold'>📋 Задачи</span>");
        vbox.pack_start(&header, false, false, 5);

        let date_label = &screens.borrow().date_label;
        date_label.set_markup("<span size='medium'>Дата: </span>");
        vbox.pack_start(date_label, false, false, 5);

        // Task list using TreeView
        let scrolled = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        scrolled.set_vexpand(true);

        let tree_view = gtk::TreeView::with_model(&screens.borrow().task_store);
        tree_view.set_headers_visible(false);

        // Text column
        let text_renderer = gtk::CellRendererText::new();
        text_renderer.set_property("wrap-mode", WrapMode::Word);
        let text_column = gtk::TreeViewColumn::new();
        text_column.set_expand(true);
        gtk::prelude::CellLayoutExt::pack_start(&text_column, &text_renderer, true);
        gtk::prelude::CellLayoutExt::add_attribute(&text_column, &text_renderer, "text", 1);
        tree_view.append_column(&text_column);

        // Status column
        let status_renderer = gtk::CellRendererText::new();
        let status_column = gtk::TreeViewColumn::new();
        gtk::prelude::CellLayoutExt::pack_start(&status_column, &status_renderer, true);
        gtk::prelude::CellLayoutExt::add_attribute(&status_column, &status_renderer, "text", 2);
        tree_view.append_column(&status_column);

        // Edit button column
        let btn_renderer = gtk::CellRendererText::new();
        btn_renderer.set_property("text", "✏️");
        let btn_column = gtk::TreeViewColumn::new();
        gtk::prelude::CellLayoutExt::pack_start(&btn_column, &btn_renderer, false);
        tree_view.append_column(&btn_column);

        scrolled.add(&tree_view);
        vbox.pack_start(&scrolled, true, true, 5);

        // Buttons
        let btn_box = GtkBox::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::Center);

        let back_btn = Button::with_label("← Назад");
        back_btn.connect_clicked({
            let stack = self.stack.clone();
            move |_| {
                stack.set_visible_child_name(SCREEN_MAIN);
            }
        });

        let add_btn = Button::with_label("+ Добавить");
        add_btn.connect_clicked({
            let stack = self.stack.clone();
            move |_| {
                stack.set_visible_child_name(SCREEN_EDIT);
            }
        });

        let copy_btn = Button::with_label("📋 Копировать отчет");
        copy_btn.connect_clicked({
            let state = self.state.clone();
            move |_| {
                let s = state.borrow();
                let report = s.generate_report(&s.cur_date);
                drop(s);

                let clipboard = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
                clipboard.set_text(&report);

                let dialog = gtk::MessageDialog::new(
                    Option::<&Window>::None,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Info,
                    gtk::ButtonsType::Ok,
                    "Отчет скопирован в буфер обмена!"
                );
                dialog.run();
                dialog.close();
            }
        });

        btn_box.pack_start(&back_btn, false, false, 5);
        btn_box.pack_start(&add_btn, false, false, 5);
        btn_box.pack_start(&copy_btn, false, false, 5);
        vbox.pack_start(&btn_box, false, false, 5);

        // Double-click to edit
        let stack_clone = self.stack.clone();
        let edit_context = self.edit_context.clone();
        let task_store_clone = screens.borrow().task_store.clone();
        tree_view.connect_row_activated(move |_, path, _| {
            let model = task_store_clone.clone();
            let iter = model.iter(path).unwrap();
            let id: u32 = model.value(&iter, 0).get().unwrap();
            
            // Store edit context with task ID only (date will be set on screen show)
            *edit_context.borrow_mut() = Some(EditContext {
                task_id: Some(id),
                date: String::new(),
            });
            
            stack_clone.set_visible_child_name(SCREEN_EDIT);
        });

        vbox.show_all();
        vbox.upcast()
    }

    fn create_edit_screen(&self, screens: &Rc<RefCell<Screens>>) -> gtk::Widget {
        let vbox = GtkBox::new(Orientation::Vertical, 10);
        vbox.set_margin_top(20);
        vbox.set_margin_bottom(20);
        vbox.set_margin_start(20);
        vbox.set_margin_end(20);

        let header = Label::new(None);
        header.set_markup("<span size='large' weight='bold'>✏️ Задача</span>");
        vbox.pack_start(&header, false, false, 10);

        let text_label = Label::new(Some("Текст задачи:"));
        text_label.set_halign(Align::Start);
        vbox.pack_start(&text_label, false, false, 5);

        let text_view = TextView::new();
        text_view.set_vexpand(true);
        text_view.set_wrap_mode(gtk::WrapMode::Word);
        let text_buffer = text_view.buffer().unwrap();
        vbox.pack_start(&text_view, true, true, 10);

        let status_label = Label::new(Some("Статус:"));
        status_label.set_halign(Align::Start);
        vbox.pack_start(&status_label, false, false, 5);

        let status_combo = gtk::ComboBoxText::new();
        for status in Status::all() {
            status_combo.append(Some(status.to_str()), status.to_str());
        }
        status_combo.set_active(Some(0));
        vbox.pack_start(&status_combo, false, false, 5);

        let date_label = Label::new(Some("Дата (ГГГГ-ММ-ДД):"));
        date_label.set_halign(Align::Start);
        vbox.pack_start(&date_label, false, false, 5);

        let date_entry = Entry::new();
        date_entry.set_text(&chrono::Local::now().format("%Y-%m-%d").to_string());
        vbox.pack_start(&date_entry, false, false, 5);

        let btn_box = GtkBox::new(Orientation::Horizontal, 10);
        btn_box.set_halign(Align::Center);
        btn_box.set_margin_top(20);

        let back_btn = Button::with_label("← Назад");
        back_btn.connect_clicked({
            let stack = self.stack.clone();
            move |_| {
                stack.set_visible_child_name(SCREEN_TASKS);
            }
        });

        let delete_btn = Button::with_label("🗑️ Удалить");
        delete_btn.connect_clicked({
            let state = self.state.clone();
            let stack = self.stack.clone();
            let edit_context = self.edit_context.clone();
            move |_| {
                // Get task info to delete
                let task_info = {
                    let ctx = edit_context.borrow();
                    if let Some(ref ctx) = *ctx {
                        if let Some(task_id) = ctx.task_id {
                            Some((task_id, ctx.date.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some((task_id, date)) = task_info {
                    let mut s = state.borrow_mut();
                    let _ = s.delete_row(date, task_id);
                    let _ = s.save();
                    drop(s);

                    let dialog = gtk::MessageDialog::new(
                        Option::<&Window>::None,
                        gtk::DialogFlags::MODAL,
                        gtk::MessageType::Info,
                        gtk::ButtonsType::Ok,
                        "Задача удалена!"
                    );
                    dialog.run();
                    dialog.close();

                    // Clear context
                    *edit_context.borrow_mut() = None;

                    stack.set_visible_child_name(SCREEN_TASKS);
                }
            }
        });

        let save_btn = Button::with_label("💾 Сохранить");
        
        let state = self.state.clone();
        let stack = self.stack.clone();
        let edit_context = self.edit_context.clone();
        let text_buffer_clone = text_buffer.clone();
        let status_combo_clone = status_combo.clone();
        let date_entry_clone = date_entry.clone();
        
        save_btn.connect_clicked(move |_| {
            let (start, end) = text_buffer_clone.bounds();
            let text = text_buffer_clone.text(&start, &end, false).unwrap_or_default().trim().to_string();

            if text.is_empty() {
                let dialog = gtk::MessageDialog::new(
                    Option::<&Window>::None,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Warning,
                    gtk::ButtonsType::Ok,
                    "Введите текст задачи!"
                );
                dialog.run();
                dialog.close();
                return;
            }

            let status_str = status_combo_clone.active_id().unwrap_or_else(|| "В работе".into());
            let status = match status_str.as_str() {
                "Отдал в тестирование" => Status::Testing,
                "Готово" => Status::Ready,
                _ => Status::Working,
            };

            let date = date_entry_clone.text().to_string();
            if date.len() != 10 || !date.chars().all(|c| c.is_digit(10) || c == '-') {
                let dialog = gtk::MessageDialog::new(
                    Option::<&Window>::None,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Warning,
                    gtk::ButtonsType::Ok,
                    "Неверный формат даты (используйте ГГГГ-ММ-ДД)!"
                );
                dialog.run();
                dialog.close();
                return;
            }

            let mut s = state.borrow_mut();
            let ctx = edit_context.borrow();
            
            if let Some(ref ctx) = *ctx {
                // Edit existing task
                if let Some(task_id) = ctx.task_id {
                    let old_date = ctx.date.clone();
                    // Update text
                    let _ = s.edit_row(old_date.clone(), task_id, text.clone());
                    // Update status
                    let _ = s.update_row_status(old_date.clone(), task_id, status);
                    // Move to new date if changed
                    if old_date != date {
                        let _ = s.move_row(old_date, task_id, date.clone());
                    }
                }
            } else {
                // Create new task
                let current_date = s.cur_date.clone();
                s.cur_date = date.clone();
                let _ = s.add_row(text.clone());
                let new_id = s.max_id;
                let _ = s.update_row_status(date.clone(), new_id, status);
                s.cur_date = current_date;
            }
            
            let _ = s.save();
            drop(s);
            drop(ctx);

            let dialog = gtk::MessageDialog::new(
                Option::<&Window>::None,
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Info,
                gtk::ButtonsType::Ok,
                "Задача сохранена!"
            );
            dialog.run();
            dialog.close();

            // Clear context
            *edit_context.borrow_mut() = None;

            // Clear form
            text_buffer_clone.set_text("");
            status_combo_clone.set_active(Some(0));
            date_entry_clone.set_text(&chrono::Local::now().format("%Y-%m-%d").to_string());

            stack.set_visible_child_name(SCREEN_TASKS);
        });

        btn_box.pack_start(&back_btn, false, false, 5);
        btn_box.pack_start(&delete_btn, false, false, 5);
        btn_box.pack_start(&save_btn, false, false, 5);
        vbox.pack_start(&btn_box, false, false, 5);

        // Store widget references for later updates
        *self.edit_widgets.borrow_mut() = Some(EditWidgets {
            header: header.clone(),
            text_buffer: text_buffer.clone(),
            status_combo: status_combo.clone(),
            date_entry: date_entry.clone(),
            delete_btn: delete_btn.clone(),
        });

        vbox.show_all();
        vbox.upcast()
    }

    pub fn run(&self) {
        self.window.show_all();
        gtk::main();
    }
}
