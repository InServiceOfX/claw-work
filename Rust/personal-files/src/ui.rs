//! Native GTK UI. Directory scans run on a bounded, single background worker.
use crate::{
    appearance,
    core::{self, Entry, Preferences, SavedTab, Sort},
};
use gtk::{gdk, gdk_pixbuf, gio, glib, pango, prelude::*};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Request {
    generation: u64,
    target: PathBuf,
    select: Option<PathBuf>,
    record: bool,
    history_index: Option<usize>,
    fallback: bool,
}
struct Scan {
    request: Request,
    result: std::io::Result<Vec<Entry>>,
}
#[derive(Clone)]
struct Navigation {
    current: PathBuf,
    history: Vec<PathBuf>,
    index: usize,
    generation: u64,
    loading: bool,
    items: Vec<Entry>,
}

#[derive(Clone)]
struct TabSnapshot {
    navigation: Navigation,
    selection: Vec<PathBuf>,
    sort: Sort,
    descending: bool,
    pending: Option<Request>,
    page: gtk::Box,
    label: gtk::Label,
    close: gtk::Button,
}

pub struct Browser {
    pub window: gtk::ApplicationWindow,
    header: gtk::HeaderBar,
    notebook: gtk::Notebook,
    tabs: RefCell<Vec<TabSnapshot>>,
    active_tab: Cell<usize>,
    tab_mutation: Cell<bool>,
    next_generation: Cell<u64>,
    pending: RefCell<Option<Request>>,
    address: gtk::Entry,
    tree: gtk::TreeView,
    store: gtk::ListStore,
    sidebar: gtk::Box,
    status: gtk::Label,
    empty: gtk::Label,
    pin: gtk::Button,
    back: gtk::Button,
    forward: gtk::Button,
    css: gtk::CssProvider,
    prefs: RefCell<Preferences>,
    nav: RefCell<Navigation>,
    displayed: RefCell<Vec<PathBuf>>,
    columns: Vec<(Sort, gtk::TreeViewColumn)>,
    sender: mpsc::Sender<Request>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    changed: Cell<Option<Instant>>,
    alive: Cell<bool>,
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(Some(icon), gtk::IconSize::Button);
    button.set_tooltip_text(Some(tooltip));
    button
}

impl Browser {
    pub fn new(app: &gtk::Application, start: Option<PathBuf>, prefs: Preferences) -> Rc<Self> {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Personal Files")
            .default_width(1080)
            .default_height(680)
            .build();
        window.set_size_request(700, 380);
        let header = gtk::HeaderBar::builder()
            .title("Personal Files")
            .subtitle("Your places, one click away")
            .show_close_button(true)
            .build();
        header.set_decoration_layout(Some("close,minimize,maximize:"));
        if let Ok(icon) = gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(
            include_bytes!("../assets/personal-files.svg").as_slice(),
        )) {
            window.set_icon(Some(&icon));
        }
        window.set_titlebar(Some(&header));
        let back = icon_button("go-previous-symbolic", "Back · Alt+Left");
        let forward = icon_button("go-next-symbolic", "Forward · Alt+Right");
        let up = icon_button("go-up-symbolic", "Parent folder · Alt+Up");
        header.pack_start(&back);
        header.pack_start(&forward);
        header.pack_start(&up);
        let skin = gtk::MenuButton::builder().label("Appearance").build();
        header.pack_end(&skin);
        let new_tab = icon_button("list-add-symbolic", "New tab · Ctrl+T");
        header.pack_end(&new_tab);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&root);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        toolbar.set_widget_name("toolbar");
        let address = gtk::Entry::builder()
            .hexpand(true)
            .placeholder_text("Go to a path — /absolute, ~/home, or relative")
            .build();
        address.set_tooltip_text(Some("Paste a path and press Enter. Relative paths start at the current folder. Ctrl+L to focus."));
        let pin = icon_button("non-starred-symbolic", "Favorite this folder · Ctrl+D");
        let copy = icon_button(
            "edit-copy-symbolic",
            "Copy selected paths, or current folder · Ctrl+C",
        );
        let refresh = icon_button("view-refresh-symbolic", "Refresh · Ctrl+R");
        toolbar.pack_start(&address, true, true, 0);
        for button in [&pin, &copy, &refresh] {
            toolbar.pack_start(button, false, false, 0);
        }
        root.pack_start(&toolbar, false, false, 0);
        let notebook = gtk::Notebook::new();
        notebook.set_scrollable(true);
        notebook.set_show_border(false);
        notebook.set_widget_name("tabs");
        root.pack_start(&notebook, false, false, 0);
        let pane = gtk::Paned::new(gtk::Orientation::Horizontal);
        pane.set_position(235);
        root.pack_start(&pane, true, true, 0);
        let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
        sidebar.set_widget_name("sidebar");
        let sidebar_scroll =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        sidebar_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        sidebar_scroll.add(&sidebar);
        pane.pack1(&sidebar_scroll, false, false);
        let canvas = gtk::Overlay::new();
        canvas.set_widget_name("canvas");
        pane.pack2(&canvas, true, false);
        let scroller = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        canvas.add(&scroller);
        let store = gtk::ListStore::new(&[
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            u32::static_type(),
        ]);
        let tree = gtk::TreeView::with_model(&store);
        tree.set_search_column(1);
        tree.set_tooltip_column(4);
        tree.selection().set_mode(gtk::SelectionMode::Multiple);
        let name = gtk::TreeViewColumn::new();
        name.set_title("Name");
        name.set_expand(true);
        name.set_min_width(180);
        let icon = gtk::CellRendererPixbuf::new();
        icon.set_property("stock-size", 3u32);
        TreeViewColumnExt::pack_start(&name, &icon, false);
        TreeViewColumnExt::add_attribute(&name, &icon, "icon-name", 0);
        let text = gtk::CellRendererText::new();
        text.set_ellipsize(pango::EllipsizeMode::Middle);
        text.set_ypad(9);
        TreeViewColumnExt::pack_start(&name, &text, true);
        TreeViewColumnExt::add_attribute(&name, &text, "text", 1);
        let mut columns = vec![(Sort::Name, name)];
        for (title, index, key, width) in [
            ("Modified", 2, Sort::Modified, 165),
            ("Size", 3, Sort::Size, 90),
        ] {
            let cell = gtk::CellRendererText::new();
            cell.set_ypad(9);
            let column = gtk::TreeViewColumn::new();
            column.set_title(title);
            TreeViewColumnExt::pack_start(&column, &cell, true);
            TreeViewColumnExt::add_attribute(&column, &cell, "text", index);
            column.set_min_width(width);
            columns.push((key, column));
        }
        for (_, column) in &columns {
            column.set_clickable(true);
            column.set_resizable(true);
            tree.append_column(column);
        }
        scroller.add(&tree);
        let empty = gtk::Label::new(Some("This folder is empty"));
        empty.set_halign(gtk::Align::Center);
        empty.set_valign(gtk::Align::Center);
        empty.set_no_show_all(true);
        canvas.add_overlay(&empty);
        canvas.set_overlay_pass_through(&empty, true);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        footer.set_widget_name("status");
        let status = gtk::Label::builder()
            .xalign(0.)
            .hexpand(true)
            .ellipsize(pango::EllipsizeMode::Middle)
            .build();
        footer.pack_start(&status, true, true, 0);
        let hint = gtk::Label::new(Some(
            "Hidden files visible · Ctrl+L: path · Ctrl+Shift+C: shell path",
        ));
        hint.style_context().add_class("hint");
        footer.pack_end(&hint, false, false, 0);
        root.pack_end(&footer, false, false, 0);
        let css = gtk::CssProvider::new();
        if let Some(screen) = WidgetExt::screen(&window) {
            gtk::StyleContext::add_provider_for_screen(
                &screen,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        let (sender, receiver) = mpsc::channel::<Request>();
        let (results, responses) = mpsc::channel::<Scan>();
        thread::spawn(move || {
            while let Ok(mut request) = receiver.recv() {
                // Coalesce pending navigations; at most one scan runs at once.
                while let Ok(newer) = receiver.try_recv() {
                    request = newer;
                }
                let result = core::entries(&request.target);
                if results.send(Scan { request, result }).is_err() {
                    break;
                }
            }
        });
        let restore_active = if start.is_some() {
            0
        } else {
            prefs.data.active_tab
        };
        let restore = if let Some(path) = start {
            vec![SavedTab {
                path,
                sort: prefs.data.sort,
                descending: prefs.data.descending,
            }]
        } else if !prefs.data.tabs.is_empty() {
            prefs.data.tabs.clone()
        } else {
            vec![SavedTab {
                path: prefs.data.last_path.clone(),
                sort: prefs.data.sort,
                descending: prefs.data.descending,
            }]
        };
        let browser = Rc::new(Self {
            window,
            header,
            notebook,
            tabs: RefCell::new(vec![]),
            active_tab: Cell::new(0),
            tab_mutation: Cell::new(true),
            next_generation: Cell::new(0),
            pending: RefCell::new(None),
            address,
            tree,
            store,
            sidebar,
            status,
            empty,
            pin,
            back,
            forward,
            css,
            prefs: RefCell::new(prefs),
            nav: RefCell::new(Navigation {
                current: core::home(),
                history: vec![],
                index: 0,
                generation: 0,
                loading: false,
                items: vec![],
            }),
            displayed: RefCell::new(vec![]),
            columns,
            sender,
            monitor: RefCell::new(None),
            changed: Cell::new(None),
            alive: Cell::new(true),
        });
        // Signals use weak references, so closing a window releases its state.
        macro_rules! connect_button {
            ($button:expr, $method:ident) => {{
                let weak = Rc::downgrade(&browser);
                $button.connect_clicked(move |_| {
                    if let Some(b) = weak.upgrade() {
                        b.$method();
                    }
                });
            }};
        }
        {
            let weak = Rc::downgrade(&browser);
            new_tab.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.new_tab(b.current());
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.notebook.connect_switch_page(move |_, _, index| {
                if let Some(b) = weak.upgrade() {
                    if !b.tab_mutation.get() {
                        b.switch_tab(index as usize);
                    }
                }
            });
        }
        connect_button!(browser.pin, toggle_favorite);
        connect_button!(refresh, refresh);
        {
            let weak = Rc::downgrade(&browser);
            copy.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.copy(false, None);
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            up.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    let parent = b.current().parent().unwrap_or(Path::new("/")).to_path_buf();
                    b.navigate(&parent, true, None, false);
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.back.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.go_history(-1);
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.forward.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.go_history(1);
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.address.connect_activate(move |entry| {
                if let Some(b) = weak.upgrade() {
                    b.go_text(&entry.text());
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.tree.selection().connect_changed(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.update_status();
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.tree.connect_row_activated(move |_, row, _| {
                if let Some(b) = weak.upgrade() {
                    if let Some(path) = b.row_path(row) {
                        b.open(&path);
                    }
                }
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.tree.connect_button_press_event(move |tree, event| {
                if event.button() != 3 {
                    return glib::Propagation::Proceed;
                }
                if let Some(b) = weak.upgrade() {
                    let (x, y) = event.position();
                    if let Some((Some(row), _, _, _)) = tree.path_at_pos(x as i32, y as i32) {
                        if !tree.selection().path_is_selected(&row) {
                            tree.selection().unselect_all();
                            tree.selection().select_path(&row);
                        }
                    } else {
                        tree.selection().unselect_all();
                    }
                    let mut paths = b.selected();
                    if paths.is_empty() {
                        paths.push(b.current());
                    }
                    b.path_menu(paths).popup_at_pointer(Some(event));
                }
                glib::Propagation::Stop
            });
        }
        for (key, column) in &browser.columns {
            let key = *key;
            let weak = Rc::downgrade(&browser);
            column.connect_clicked(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.sort(key);
                }
            });
        }
        let targets = [
            gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::OTHER_APP, 0),
            gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::SAME_APP, 0),
        ];
        browser.tree.drag_source_set(
            gdk::ModifierType::BUTTON1_MASK,
            &targets,
            gdk::DragAction::COPY,
        );
        {
            let weak = Rc::downgrade(&browser);
            browser.tree.connect_drag_data_get(move |_, _, data, _, _| {
                if let Some(b) = weak.upgrade() {
                    let uris = b.drag_uris();
                    let refs: Vec<_> = uris.iter().map(String::as_str).collect();
                    data.set_uris(&refs);
                }
            });
        }
        browser
            .sidebar
            .drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::COPY);
        {
            let weak = Rc::downgrade(&browser);
            browser
                .sidebar
                .connect_drag_data_received(move |_, context, _, _, data, _, time| {
                    let mut success = false;
                    if let Some(b) = weak.upgrade() {
                        let paths: Vec<_> = data
                            .uris()
                            .iter()
                            .filter_map(|uri| gio::File::for_uri(uri).path())
                            .collect();
                        success = b.add_favorites(&paths) > 0;
                    }
                    context.drag_finish(success, false, time);
                });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.window.connect_key_press_event(move |_, event| {
                weak.upgrade()
                    .map_or(glib::Propagation::Proceed, |b| b.key(event))
            });
        }
        {
            let weak = Rc::downgrade(&browser);
            browser.window.connect_destroy(move |_| {
                if let Some(b) = weak.upgrade() {
                    b.alive.set(false);
                    if let Some(m) = b.monitor.borrow_mut().take() {
                        m.cancel();
                    }
                    if let Some(screen) = WidgetExt::screen(&b.window) {
                        gtk::StyleContext::remove_provider_for_screen(&screen, &b.css);
                    }
                }
            });
        }
        skin.set_popup(Some(&browser.appearance_menu()));
        browser.rebuild_sidebar();
        browser.apply_appearance();
        browser.window.show_all();
        let weak = Rc::downgrade(&browser);
        glib::timeout_add_local(Duration::from_millis(30), move || {
            let Some(b) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if !b.alive.get() {
                return glib::ControlFlow::Break;
            }
            while let Ok(scan) = responses.try_recv() {
                b.finish(scan);
            }
            if b.changed
                .get()
                .is_some_and(|t| t.elapsed() > Duration::from_millis(250))
                && !b.nav.borrow().loading
            {
                b.changed.set(None);
                b.refresh();
            }
            glib::ControlFlow::Continue
        });
        for tab in restore {
            browser.append_tab(tab.path, tab.sort, tab.descending);
        }
        browser.tab_mutation.set(false);
        let restore_active = restore_active.min(browser.tabs.borrow().len() - 1);
        browser.tab_mutation.set(true);
        browser
            .notebook
            .set_current_page(Some(restore_active as u32));
        browser.tab_mutation.set(false);
        browser.switch_tab(restore_active);
        browser.notebook.show_all();
        browser
    }

    pub fn current(&self) -> PathBuf {
        self.nav.borrow().current.clone()
    }

    fn append_tab(self: &Rc<Self>, path: PathBuf, sort: Sort, descending: bool) {
        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let title = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        title.set_size_request(180, -1);
        let label = gtk::Label::new(Some(
            &path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy(),
        ));
        label.set_ellipsize(pango::EllipsizeMode::Middle);
        label.set_max_width_chars(24);
        let close = icon_button("window-close-symbolic", "Close tab · Ctrl+W");
        close.set_relief(gtk::ReliefStyle::None);
        title.pack_start(&label, true, true, 0);
        title.pack_end(&close, false, false, 0);
        let weak = Rc::downgrade(self);
        let weak_page = page.downgrade();
        close.connect_clicked(move |_| {
            if let (Some(browser), Some(page)) = (weak.upgrade(), weak_page.upgrade()) {
                if let Some(index) = browser.notebook.page_num(&page) {
                    browser.close_tab(index as usize);
                }
            }
        });
        let previous = self.tab_mutation.replace(true);
        self.tabs.borrow_mut().push(TabSnapshot {
            navigation: Navigation {
                current: path.clone(),
                history: vec![],
                index: 0,
                generation: 0,
                loading: false,
                items: vec![],
            },
            selection: vec![],
            sort,
            descending,
            pending: Some(Request {
                generation: 0,
                target: path,
                select: None,
                record: true,
                history_index: None,
                fallback: true,
            }),
            page: page.clone(),
            label,
            close,
        });
        self.notebook.append_page(&page, Some(&title));
        page.show_all();
        title.show_all();
        self.tab_mutation.set(previous);
        self.update_tab_labels();
    }

    fn capture_tab(&self) {
        if self.nav.borrow().history.is_empty() && self.pending.borrow().is_none() {
            return;
        }
        let (sort, descending) = {
            let p = self.prefs.borrow();
            (p.data.sort, p.data.descending)
        };
        let selection = self.selected();
        if let Some(tab) = self.tabs.borrow_mut().get_mut(self.active_tab.get()) {
            tab.navigation = self.nav.borrow().clone();
            tab.selection = selection;
            tab.sort = sort;
            tab.descending = descending;
            tab.pending = self.pending.borrow().clone();
        }
    }

    fn persist_session(&self) {
        let tabs = self.tabs.borrow();
        if tabs.is_empty() {
            return;
        }
        let mut prefs = self.prefs.borrow_mut();
        prefs.data.tabs = tabs
            .iter()
            .map(|tab| SavedTab {
                // Keep the successful folder when new path input has not been validated.
                path: tab.navigation.current.clone(),
                sort: tab.sort,
                descending: tab.descending,
            })
            .collect();
        prefs.data.active_tab = self.active_tab.get();
        prefs.data.last_path = self.current();
    }

    fn update_tab_labels(&self) {
        let tabs = self.tabs.borrow();
        for (index, tab) in tabs.iter().enumerate() {
            let path = if index == self.active_tab.get() && !self.nav.borrow().history.is_empty() {
                self.current()
            } else {
                tab.navigation.current.clone()
            };
            tab.label.set_text(
                &path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy(),
            );
            tab.page.set_tooltip_text(Some(&path.to_string_lossy()));
            tab.label.set_tooltip_text(Some(&path.to_string_lossy()));
            tab.close.set_sensitive(tabs.len() > 1);
        }
    }

    fn switch_tab(self: &Rc<Self>, index: usize) {
        if index >= self.tabs.borrow().len() {
            return;
        }
        self.capture_tab();
        let incoming = self.tabs.borrow()[index].clone();
        self.tree.selection().unselect_all();
        self.active_tab.set(index);
        *self.nav.borrow_mut() = incoming.navigation;
        // Never reuse a generation from another tab: old worker results may still arrive.
        self.next_generation.set(self.next_generation.get() + 1);
        self.nav.borrow_mut().generation = self.next_generation.get();
        self.nav.borrow_mut().loading = false;
        *self.pending.borrow_mut() = None;
        {
            let mut p = self.prefs.borrow_mut();
            p.data.sort = incoming.sort;
            p.data.descending = incoming.descending;
        }
        self.store.clear();
        self.populate(None);
        for (i, path) in self.displayed.borrow().iter().enumerate() {
            if incoming.selection.contains(path) {
                self.tree
                    .selection()
                    .select_path(&gtk::TreePath::from_indicesv(&[i as i32]));
            }
        }
        self.address.set_text(&self.current().to_string_lossy());
        self.header.set_subtitle(Some(
            &self
                .current()
                .file_name()
                .unwrap_or(self.current().as_os_str())
                .to_string_lossy(),
        ));
        {
            let n = self.nav.borrow();
            self.back.set_sensitive(n.index > 0);
            self.forward.set_sensitive(n.index + 1 < n.history.len());
        }
        self.rebuild_sidebar();
        self.apply_appearance();
        self.update_tab_labels();
        if let Some(monitor) = self.monitor.borrow_mut().take() {
            monitor.cancel();
        }
        self.changed.set(None);
        if let Some(request) = incoming.pending {
            self.navigate(
                request.select.as_ref().unwrap_or(&request.target),
                request.record,
                request.history_index,
                request.fallback,
            );
        } else {
            self.refresh();
        }
        self.save();
    }

    fn new_tab(self: &Rc<Self>, path: PathBuf) {
        let (sort, descending) = {
            let p = self.prefs.borrow();
            (p.data.sort, p.data.descending)
        };
        self.append_tab(path, sort, descending);
        let index = self.tabs.borrow().len() - 1;
        self.notebook.set_current_page(Some(index as u32));
    }

    fn close_tab(self: &Rc<Self>, index: usize) {
        let len = self.tabs.borrow().len();
        if len <= 1 || index >= len {
            return;
        }
        if index == self.active_tab.get() {
            let next = if index + 1 < len {
                index + 1
            } else {
                index - 1
            };
            self.switch_tab(next);
        }
        self.capture_tab();
        self.tab_mutation.set(true);
        self.tabs.borrow_mut().remove(index);
        self.notebook.remove_page(Some(index as u32));
        if self.active_tab.get() > index {
            self.active_tab.set(self.active_tab.get() - 1);
        }
        self.notebook
            .set_current_page(Some(self.active_tab.get() as u32));
        self.tab_mutation.set(false);
        self.update_tab_labels();
        self.save();
    }

    fn tab_shortcut(self: &Rc<Self>, key: gdk::keys::Key, modifiers: gdk::ModifierType) -> bool {
        if !modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            return false;
        }
        let lower = key.to_lower();
        if lower == gdk::keys::constants::t {
            self.new_tab(self.current());
        } else if lower == gdk::keys::constants::w {
            self.close_tab(self.active_tab.get());
        } else if key == gdk::keys::constants::Tab || key == gdk::keys::constants::ISO_Left_Tab {
            let len = self.tabs.borrow().len();
            let reverse = modifiers.contains(gdk::ModifierType::SHIFT_MASK)
                || key == gdk::keys::constants::ISO_Left_Tab;
            let index = (self.active_tab.get() + if reverse { len - 1 } else { 1 }) % len;
            self.notebook.set_current_page(Some(index as u32));
        } else {
            return false;
        }
        true
    }
    fn message(&self, text: &str) {
        self.status.set_text(text);
        self.status.set_tooltip_text(Some(text));
    }
    fn save(&self) {
        self.capture_tab();
        self.persist_session();
        let result = self.prefs.borrow_mut().save();
        if let Err(e) = result {
            self.message(&format!("Could not save preferences: {e}"));
        }
    }
    fn go_text(self: &Rc<Self>, text: &str) {
        match core::location(text, &self.current()) {
            Ok(path) => self.navigate(&path, true, None, false),
            Err(e) => self.message(&e.to_string()),
        }
    }
    fn navigate(
        self: &Rc<Self>,
        path: &Path,
        record: bool,
        history_index: Option<usize>,
        fallback: bool,
    ) {
        let (target, select) = if path.is_file() {
            (
                path.parent().unwrap_or(Path::new("/")).to_path_buf(),
                Some(path.to_path_buf()),
            )
        } else {
            (path.to_path_buf(), None)
        };
        let mut nav = self.nav.borrow_mut();
        self.next_generation.set(self.next_generation.get() + 1);
        nav.generation = self.next_generation.get();
        nav.loading = true;
        let request = Request {
            generation: nav.generation,
            target: target.clone(),
            select,
            record,
            history_index,
            fallback,
        };
        drop(nav);
        self.message(&format!("Opening {} …", target.display()));
        *self.pending.borrow_mut() = Some(request.clone());
        if self.sender.send(request).is_err() {
            self.nav.borrow_mut().loading = false;
            self.message("The folder worker stopped. Please reopen Personal Files.");
        }
    }
    fn finish(self: &Rc<Self>, scan: Scan) {
        if scan.request.generation != self.nav.borrow().generation {
            return;
        }
        self.nav.borrow_mut().loading = false;
        self.pending.borrow_mut().take();
        let request = scan.request;
        match scan.result {
            Err(e) => {
                self.address.set_text(&self.current().to_string_lossy());
                self.message(&format!("Cannot open folder: {e}"));
                if request.fallback && request.target != core::home() {
                    self.navigate(&core::home(), true, None, false);
                }
            }
            Ok(items) => {
                {
                    let mut nav = self.nav.borrow_mut();
                    nav.current = request.target.clone();
                    nav.items = items;
                    if let Some(index) = request.history_index {
                        nav.index = index;
                    } else if nav.history.is_empty()
                        || (request.record && nav.history.get(nav.index) != Some(&request.target))
                    {
                        let next = if nav.history.is_empty() {
                            0
                        } else {
                            nav.index + 1
                        };
                        nav.history.truncate(next);
                        nav.history.push(request.target.clone());
                        nav.index = nav.history.len() - 1;
                    }
                }
                self.address.set_text(&request.target.to_string_lossy());
                self.header.set_subtitle(Some(
                    &request
                        .target
                        .file_name()
                        .unwrap_or(request.target.as_os_str())
                        .to_string_lossy(),
                ));
                if request.record {
                    self.prefs.borrow_mut().visit(&request.target);
                }
                self.populate(request.select);
                self.rebuild_sidebar();
                self.apply_appearance();
                if request.record {
                    self.save();
                }
                {
                    let nav = self.nav.borrow();
                    self.back.set_sensitive(nav.index > 0);
                    self.forward
                        .set_sensitive(nav.index + 1 < nav.history.len());
                }
                self.update_tab_labels();
                self.watch();
                let warning = self.prefs.borrow_mut().warning.take();
                if let Some(warning) = warning {
                    self.message(&warning);
                }
            }
        }
    }
    fn populate(&self, select: Option<PathBuf>) {
        let selected = select.map(|p| vec![p]).unwrap_or_else(|| self.selected());
        let (key, descending) = {
            let p = self.prefs.borrow();
            (p.data.sort, p.data.descending)
        };
        let mut items = self.nav.borrow().items.clone();
        core::sort_entries(&mut items, key, descending);
        // Set lookup before model changes trigger selection signals.
        self.store.clear();
        *self.displayed.borrow_mut() = items.iter().map(|e| e.path.clone()).collect();
        for (index, entry) in items.iter().enumerate() {
            let icon = if entry.is_dir {
                "folder"
            } else if entry.symlink {
                "emblem-symbolic-link"
            } else {
                "text-x-generic"
            };
            let modified = glib::DateTime::from_unix_local(entry.modified as i64)
                .and_then(|d| d.format("%b %d, %Y  %H:%M"))
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "—".into());
            let size = if entry.is_dir {
                "—".into()
            } else {
                core::size_label(entry.size)
            };
            let path = entry.path.to_string_lossy().into_owned();
            let name = entry.name();
            let iter = self.store.insert_with_values(
                None,
                &[
                    (0, &icon),
                    (1, &name),
                    (2, &modified),
                    (3, &size),
                    (4, &path),
                    (5, &(index as u32)),
                ],
            );
            if selected.contains(&entry.path) {
                self.tree.selection().select_iter(&iter);
            }
        }
        self.empty.set_visible(items.is_empty());
        self.update_status();
        for (sort, column) in &self.columns {
            column.set_sort_indicator(*sort == key);
            column.set_sort_order(if descending {
                gtk::SortType::Descending
            } else {
                gtk::SortType::Ascending
            });
        }
    }
    fn row_path(&self, row: &gtk::TreePath) -> Option<PathBuf> {
        let iter = self.store.iter(row)?;
        let index = self.store.value(&iter, 5).get::<u32>().ok()?;
        self.displayed.borrow().get(index as usize).cloned()
    }
    fn selected(&self) -> Vec<PathBuf> {
        self.tree
            .selection()
            .selected_rows()
            .0
            .iter()
            .filter_map(|row| self.row_path(row))
            .collect()
    }
    fn update_status(&self) {
        let n = self.selected().len();
        let nav = self.nav.borrow();
        self.message(&format!(
            "{} items{}  ·  {}",
            nav.items.len(),
            if n > 0 {
                format!(" · {n} selected")
            } else {
                String::new()
            },
            nav.current.display()
        ));
    }
    fn sort(&self, key: Sort) {
        {
            let mut p = self.prefs.borrow_mut();
            p.data.descending = if p.data.sort == key {
                !p.data.descending
            } else {
                key != Sort::Name
            };
            p.data.sort = key;
        }
        self.populate(None);
        self.save();
    }
    fn refresh(self: &Rc<Self>) {
        self.navigate(&self.current(), false, None, false);
    }
    fn go_history(self: &Rc<Self>, delta: isize) {
        let next = {
            let n = self.nav.borrow();
            let i = n.index as isize + delta;
            if i < 0 {
                None
            } else {
                n.history.get(i as usize).cloned().map(|p| (i as usize, p))
            }
        };
        if let Some((i, path)) = next {
            self.navigate(&path, true, Some(i), false);
        }
    }
    fn watch(self: &Rc<Self>) {
        if let Some(m) = self.monitor.borrow_mut().take() {
            m.cancel();
        }
        if let Ok(m) = gio::File::for_path(self.current())
            .monitor_directory(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
        {
            let weak = Rc::downgrade(self);
            m.connect_changed(move |_, _, _, _| {
                if let Some(b) = weak.upgrade() {
                    b.changed.set(Some(Instant::now()));
                }
            });
            *self.monitor.borrow_mut() = Some(m);
        }
    }
    fn open(self: &Rc<Self>, path: &Path) {
        if path.is_dir() {
            self.navigate(path, true, None, false);
        } else if let Err(e) = gio::AppInfo::launch_default_for_uri(
            &gio::File::for_path(path).uri(),
            None::<&gio::AppLaunchContext>,
        ) {
            self.message(&format!("Could not open file: {e}"));
        }
    }
    fn copy(&self, shell: bool, paths: Option<Vec<PathBuf>>) -> String {
        let mut paths = paths.unwrap_or_else(|| self.selected());
        if paths.is_empty() {
            paths.push(self.current());
        }
        let text = core::clipboard_text(&paths, shell);
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        clipboard.set_text(&text);
        clipboard.store();
        self.message(&format!(
            "Copied {} {}path(s)",
            paths.len(),
            if shell { "shell-quoted " } else { "full " }
        ));
        text
    }
    fn toggle_favorite(self: &Rc<Self>) {
        let current = self.current();
        {
            let mut p = self.prefs.borrow_mut();
            if p.data.favorites.contains(&current) {
                p.unfavorite(&current);
            } else {
                p.favorite(&current);
            }
        }
        self.rebuild_sidebar();
        self.save();
    }
    fn add_favorites(self: &Rc<Self>, paths: &[PathBuf]) -> usize {
        let mut count = 0;
        {
            let mut p = self.prefs.borrow_mut();
            for path in paths {
                if path.is_dir() {
                    p.favorite(path);
                    count += 1;
                }
            }
        }
        self.rebuild_sidebar();
        self.save();
        self.message(&format!("Added {count} folder(s) to Favorites"));
        count
    }
    fn drag_uris(&self) -> Vec<String> {
        self.selected()
            .iter()
            .map(|p| gio::File::for_path(p).uri().to_string())
            .collect()
    }
    fn rebuild_sidebar(self: &Rc<Self>) {
        for child in self.sidebar.children() {
            self.sidebar.remove(&child);
        }
        let (favorites, recent, frequent) = {
            let p = self.prefs.borrow();
            (p.data.favorites.clone(), p.recent(7), p.frequent(5))
        };
        self.section(
            "PLACES",
            vec![
                core::home(),
                core::home().join("Desktop"),
                core::home().join("Downloads"),
                PathBuf::from("/"),
            ]
            .into_iter()
            .filter(|p| p.is_dir())
            .collect(),
        );
        self.section("FAVORITES", favorites.clone());
        if favorites.is_empty() {
            let hint = gtk::Label::new(Some("Drag folders here, or use ☆"));
            hint.set_xalign(0.);
            hint.set_margin_start(14);
            hint.style_context().add_class("hint");
            self.sidebar.pack_start(&hint, false, false, 5);
        }
        self.section("RECENT", recent);
        self.section("FREQUENT", frequent);
        self.pin.set_image(Some(&gtk::Image::from_icon_name(
            Some(if favorites.contains(&self.current()) {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            }),
            gtk::IconSize::Button,
        )));
        self.sidebar.show_all();
    }
    fn section(self: &Rc<Self>, title: &str, paths: Vec<PathBuf>) {
        let heading = gtk::Label::new(Some(title));
        heading.set_xalign(0.);
        heading.style_context().add_class("section");
        self.sidebar.pack_start(&heading, false, false, 0);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.set_activate_on_single_click(true);
        {
            let weak = Rc::downgrade(self);
            let destinations = paths.clone();
            list.connect_row_activated(move |_, row| {
                if let (Some(browser), Some(path)) =
                    (weak.upgrade(), destinations.get(row.index() as usize))
                {
                    if browser.current() == *path {
                        browser.refresh();
                    } else {
                        browser.navigate(path, true, None, false);
                    }
                }
            });
        }
        for path in paths {
            let row = gtk::ListBoxRow::new();
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 9);
            line.pack_start(
                &gtk::Image::from_icon_name(
                    Some(if path == core::home() {
                        "user-home-symbolic"
                    } else {
                        "folder-symbolic"
                    }),
                    gtk::IconSize::Menu,
                ),
                false,
                false,
                0,
            );
            let name = if path == core::home() {
                "Home".into()
            } else if path == Path::new("/") {
                "Computer".into()
            } else {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            };
            let label = gtk::Label::new(Some(&name));
            label.set_xalign(0.);
            label.set_ellipsize(pango::EllipsizeMode::Middle);
            label.set_max_width_chars(22);
            line.pack_start(&label, true, true, 0);
            row.add(&line);
            row.set_tooltip_text(Some(&path.to_string_lossy()));
            {
                let weak = Rc::downgrade(self);
                let path = path.clone();
                row.connect_button_press_event(move |_, event| {
                    if event.button() == 3 {
                        if let Some(b) = weak.upgrade() {
                            b.path_menu(vec![path.clone()])
                                .popup_at_pointer(Some(event));
                        }
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                });
            }
            list.add(&row);
        }
        self.sidebar.pack_start(&list, false, false, 0);
    }
    fn menu_item(menu: &gtk::Menu, title: &str, callback: impl Fn() + 'static) {
        let item = gtk::MenuItem::with_label(title);
        item.connect_activate(move |_| callback());
        menu.append(&item);
    }
    fn path_menu(self: &Rc<Self>, paths: Vec<PathBuf>) -> gtk::Menu {
        let menu = gtk::Menu::new();
        menu.set_attach_widget(Some(&self.window));
        {
            let weak = Rc::downgrade(self);
            let paths = paths.clone();
            Self::menu_item(&menu, "Open", move || {
                if let Some(b) = weak.upgrade() {
                    b.open(&paths[0]);
                }
            });
        }
        for (title, shell) in [("Copy Full Path", false), ("Copy Shell-Quoted Path", true)] {
            let weak = Rc::downgrade(self);
            let paths = paths.clone();
            Self::menu_item(&menu, title, move || {
                if let Some(b) = weak.upgrade() {
                    b.copy(shell, Some(paths.clone()));
                }
            });
        }
        if paths.iter().any(|p| p.is_dir()) {
            let weak = Rc::downgrade(self);
            let folders: Vec<_> = paths.iter().filter(|p| p.is_dir()).cloned().collect();
            Self::menu_item(&menu, "Open in New Tab", move || {
                if let Some(b) = weak.upgrade() {
                    for path in &folders {
                        b.new_tab(path.clone());
                    }
                }
            });
        }
        if paths.iter().any(|p| p.is_dir()) {
            let weak = Rc::downgrade(self);
            let paths = paths.clone();
            Self::menu_item(&menu, "Add to Favorites", move || {
                if let Some(b) = weak.upgrade() {
                    b.add_favorites(&paths);
                }
            });
        }
        if paths
            .iter()
            .any(|p| self.prefs.borrow().data.favorites.contains(p))
        {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, "Remove from Favorites", move || {
                if let Some(b) = weak.upgrade() {
                    {
                        let mut p = b.prefs.borrow_mut();
                        for path in &paths {
                            p.unfavorite(path);
                        }
                    }
                    b.rebuild_sidebar();
                    b.save();
                }
            });
        }
        menu.connect_selection_done(|m| m.detach());
        menu.show_all();
        menu
    }
    fn appearance_menu(self: &Rc<Self>) -> gtk::Menu {
        let menu = gtk::Menu::new();
        for (key, title) in [
            ("silver", "Silver"),
            ("midnight", "Midnight"),
            ("aurora", "Aurora"),
            ("auto", "Automatic · match folder contents"),
        ] {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, title, move || {
                if let Some(b) = weak.upgrade() {
                    b.set_theme(key);
                }
            });
        }
        menu.append(&gtk::SeparatorMenuItem::new());
        {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, "Choose Background Image…", move || {
                if let Some(b) = weak.upgrade() {
                    b.choose_image();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, "Remove Background Image", move || {
                if let Some(b) = weak.upgrade() {
                    b.set_image(Path::new(""));
                }
            });
        }
        menu.append(&gtk::SeparatorMenuItem::new());
        {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, "Keyboard Shortcuts", move || {
                if let Some(b) = weak.upgrade() {
                    b.shortcuts();
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            Self::menu_item(&menu, "Clear Visit History", move || {
                if let Some(b) = weak.upgrade() {
                    b.prefs.borrow_mut().data.visits.clear();
                    b.rebuild_sidebar();
                    b.save();
                    b.message("Visit history cleared. Favorites kept.");
                }
            });
        }
        menu.show_all();
        menu
    }
    fn set_theme(&self, theme: &str) {
        self.prefs.borrow_mut().data.theme = theme.into();
        self.apply_appearance();
        self.save();
    }
    fn set_image(&self, path: &Path) {
        if !path.as_os_str().is_empty() {
            if let Err(e) = gdk_pixbuf::Pixbuf::from_file_at_scale(path, 64, 64, true) {
                self.message(&format!("Cannot use this image: {e}"));
                return;
            }
        }
        self.prefs.borrow_mut().data.image = path.into();
        self.apply_appearance();
        self.save();
    }
    fn choose_image(&self) {
        let dialog = gtk::FileChooserDialog::with_buttons(
            Some("Choose a background image"),
            Some(&self.window),
            gtk::FileChooserAction::Open,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Use Image", gtk::ResponseType::Ok),
            ],
        );
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Images"));
        filter.add_pixbuf_formats();
        dialog.add_filter(filter);
        if dialog.run() == gtk::ResponseType::Ok {
            if let Some(path) = dialog.filename() {
                self.set_image(&path);
            }
        }
        dialog.close();
    }
    fn apply_appearance(&self) {
        let (theme, image) = {
            let p = self.prefs.borrow();
            (
                appearance::effective_theme(&p.data.theme, &self.nav.borrow().items),
                if p.data.image.is_file() {
                    p.data.image.clone()
                } else {
                    PathBuf::new()
                },
            )
        };
        if let Err(e) = self
            .css
            .load_from_data(appearance::stylesheet(&theme, &image).as_bytes())
        {
            self.message(&format!("Appearance could not load: {e}"));
        }
    }
    fn shortcuts(&self) {
        let dialog = gtk::MessageDialog::new(
            Some(&self.window),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Close,
            "Personal Files · Keyboard Shortcuts",
        );
        dialog.set_secondary_text(Some("Ctrl+T     New tab in current folder\nCtrl+W     Close tab (keep one open)\nCtrl+Tab / Ctrl+Shift+Tab     Next / previous tab\nCtrl+L     Focus path bar\nCtrl+C     Copy full path(s)\nCtrl+Shift+C     Copy shell-quoted path(s)\nCtrl+D     Toggle favorite folder\nCtrl+R / F5     Refresh\nAlt+Left / Right     Back / forward\nAlt+Up     Parent folder\nEnter     Open selected item\n\nDrop folders into the sidebar to favorite them.\nClick Name, Modified, or Size again to reverse sorting.\nHidden files are always visible."));
        dialog.run();
        dialog.close();
    }
    fn key(self: &Rc<Self>, event: &gdk::EventKey) -> glib::Propagation {
        let ctrl = event.state().contains(gdk::ModifierType::CONTROL_MASK);
        let shift = event.state().contains(gdk::ModifierType::SHIFT_MASK);
        let alt = event.state().contains(gdk::ModifierType::MOD1_MASK);
        let key = event.keyval().to_lower();
        if self.tab_shortcut(event.keyval(), event.state()) {
            return glib::Propagation::Stop;
        }
        if ctrl && key == gdk::keys::constants::l {
            self.address.grab_focus();
            self.address.select_region(0, -1);
            return glib::Propagation::Stop;
        }
        if ctrl && key == gdk::keys::constants::c {
            if self.address.has_focus() {
                return glib::Propagation::Proceed;
            }
            self.copy(shift, None);
        } else if ctrl && key == gdk::keys::constants::d {
            self.toggle_favorite();
        } else if (ctrl && key == gdk::keys::constants::r) || key == gdk::keys::constants::F5 {
            self.refresh();
        } else if alt && key == gdk::keys::constants::Left {
            self.go_history(-1);
        } else if alt && key == gdk::keys::constants::Right {
            self.go_history(1);
        } else if alt && key == gdk::keys::constants::Up {
            let parent = self
                .current()
                .parent()
                .unwrap_or(Path::new("/"))
                .to_path_buf();
            self.navigate(&parent, true, None, false);
        } else if !self.address.has_focus()
            && (key == gdk::keys::constants::Return || key == gdk::keys::constants::KP_Enter)
        {
            if let Some(path) = self.selected().first() {
                self.open(path);
            }
        } else {
            return glib::Propagation::Proceed;
        }
        glib::Propagation::Stop
    }
}

pub fn run() -> i32 {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--help" || s == "-h") {
        println!("Personal Files — Ubuntu file browser\nUsage: personal-files [PATH]");
        return 0;
    }
    if args.len() > 1 {
        eprintln!("Usage: personal-files [PATH]");
        return 2;
    }
    if let Err(e) = gtk::init() {
        eprintln!("Personal Files needs an Ubuntu graphical desktop: {e}");
        return 1;
    }
    let start = args.first().map(PathBuf::from).map(|p| {
        if p.is_absolute() {
            p
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| core::home())
                .join(p)
        }
    });
    let app = gtk::Application::new(
        Some("io.monoclaw.PersonalFiles"),
        gio::ApplicationFlags::NON_UNIQUE,
    );
    let windows: Rc<RefCell<Vec<Rc<Browser>>>> = Rc::new(RefCell::new(vec![]));
    let keep = windows.clone();
    app.connect_activate(move |app| {
        let browser = Browser::new(
            app,
            start.clone(),
            Preferences::load(Preferences::default_path()),
        );
        keep.borrow_mut().push(browser);
    });
    app.run_with_args::<&str>(&[]).value()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn drain() {
        let context = glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
    }
    fn screenshot(browser: &Browser, path: &Path) {
        drain();
        thread::sleep(Duration::from_millis(100));
        drain();
        // Render into our own raster surface: do not mutate/capture a live X11 surface.
        let width = browser.window.allocated_width();
        let height = browser.window.allocated_height();
        let surface =
            gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height).unwrap();
        let context = gtk::cairo::Context::new(&surface).unwrap();
        browser.window.draw(&context);
        surface.flush();
        gdk::pixbuf_get_from_surface(&surface, 0, 0, width, height)
            .unwrap()
            .savev(path, "png", &[])
            .unwrap();
    }
    fn wait(browser: &Browser) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while browser.nav.borrow().loading && Instant::now() < deadline {
            drain();
            thread::sleep(Duration::from_millis(10));
        }
        drain();
        assert!(!browser.nav.borrow().loading, "folder scan timed out");
    }
    fn select(browser: &Browser, name: &str) -> gtk::TreePath {
        browser.tree.selection().unselect_all();
        let index = browser
            .displayed
            .borrow()
            .iter()
            .position(|p| p.file_name().unwrap_or_default() == name)
            .unwrap();
        let row = gtk::TreePath::from_indicesv(&[index as i32]);
        browser.tree.selection().select_path(&row);
        row
    }

    fn sidebar_story(browser: &Rc<Browser>, root: &Path, physics: &Path) {
        let downloads = root.join("Downloads");
        fs::create_dir(&downloads).unwrap();
        fs::write(downloads.join("download-marker.txt"), "fixture").unwrap();
        browser.new_tab(root.into());
        wait(browser);
        let active = browser.active_tab.get();
        // Use the same section builder as Places, with a temporary Downloads fixture.
        browser.section("TEST PLACES", vec![downloads.clone()]);
        browser.sidebar.show_all();
        drain();
        let list = browser
            .sidebar
            .children()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::ListBox>().ok())
            .last()
            .unwrap();
        let row = list.row_at_index(0).unwrap();
        assert!(list.activates_on_single_click());
        list.emit_by_name::<()>("row-activated", &[&row]);
        wait(browser);
        assert_eq!(browser.active_tab.get(), active);
        assert_eq!(browser.current(), downloads);
        assert_eq!(browser.address.text().as_str(), downloads.to_str().unwrap());
        assert!(browser
            .displayed
            .borrow()
            .contains(&downloads.join("download-marker.txt")));
        assert_eq!(browser.tabs.borrow()[0].navigation.current, physics);
        let count = browser.prefs.borrow().data.visits[&downloads].count;
        // A repeated click on the current place refreshes externally changed contents.
        fs::write(downloads.join("second-marker.txt"), "fixture").unwrap();
        // Rebuild the temporary place after the normal sidebar refresh.
        browser.section("TEST PLACES", vec![downloads.clone()]);
        browser.sidebar.show_all();
        let list = browser
            .sidebar
            .children()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::ListBox>().ok())
            .last()
            .unwrap();
        let row = list.row_at_index(0).unwrap();
        list.emit_by_name::<()>("row-activated", &[&row]);
        list.emit_by_name::<()>("row-activated", &[&row]);
        wait(browser);
        assert!(browser
            .displayed
            .borrow()
            .contains(&downloads.join("second-marker.txt")));
        assert_eq!(browser.prefs.borrow().data.visits[&downloads].count, count);
        assert_eq!(browser.tabs.borrow().len(), 2);
        browser.close_tab(active);
        wait(browser);
        assert_eq!(browser.current(), physics);
        eprintln!("PASS sidebar list mouse-activation signal: active tab, contents, repeat refresh and background tab isolation");
    }

    fn tabs_story(
        app: &gtk::Application,
        browser: &Rc<Browser>,
        root: &Path,
        physics: &Path,
        prefs_path: &Path,
    ) {
        let ctrl = gdk::ModifierType::CONTROL_MASK;
        let shift = gdk::ModifierType::SHIFT_MASK;
        browser.columns[0].1.clicked();
        if browser.prefs.borrow().data.descending {
            browser.columns[0].1.clicked();
        }
        select(browser, ".hidden");
        browser.address.grab_focus();
        assert!(browser.tab_shortcut(gdk::keys::constants::t, ctrl));
        wait(browser);
        assert_eq!(browser.tabs.borrow().len(), 2);
        assert_eq!(browser.current(), physics);
        assert_eq!(browser.active_tab.get(), 1);
        browser.navigate(&physics.join("Topics"), true, None, false);
        wait(browser);
        browser.columns[2].1.clicked();
        assert_eq!(browser.prefs.borrow().data.sort, Sort::Size);
        let visits = browser.prefs.borrow().data.visits.clone();
        assert!(browser.tab_shortcut(gdk::keys::constants::ISO_Left_Tab, ctrl | shift));
        wait(browser);
        assert_eq!(browser.current(), physics);
        assert_eq!(browser.selected(), vec![physics.join(".hidden")]);
        assert_eq!(browser.prefs.borrow().data.sort, Sort::Name);
        assert!(!browser.prefs.borrow().data.descending);
        assert!(browser.tab_shortcut(gdk::keys::constants::Tab, ctrl));
        wait(browser);
        assert_eq!(browser.current(), physics.join("Topics"));
        assert_eq!(browser.prefs.borrow().data.sort, Sort::Size);
        assert_eq!(browser.prefs.borrow().data.visits.len(), visits.len());
        for (path, visit) in visits {
            assert_eq!(browser.prefs.borrow().data.visits[&path].count, visit.count);
        }
        browser.go_history(-1);
        wait(browser);
        assert_eq!(browser.current(), physics);
        browser.go_history(1);
        wait(browser);
        assert_eq!(browser.current(), physics.join("Topics"));
        eprintln!(
            "PASS tabs: Ctrl+T, cycling, selection, sort, independent history and no extra visits"
        );

        let menu = browser.path_menu(vec![root.into()]);
        menu.children()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::MenuItem>().ok())
            .find(|i| i.label().as_deref() == Some("Open in New Tab"))
            .unwrap()
            .emit_activate();
        menu.detach();
        wait(browser);
        assert_eq!(browser.tabs.borrow().len(), 3);
        assert_eq!(browser.current(), root);
        // Closing a background tab must not navigate or reset the foreground tab.
        let close = browser.tabs.borrow()[1].close.clone();
        close.emit_clicked();
        wait(browser);
        assert_eq!(browser.tabs.borrow().len(), 2);
        assert_eq!(browser.current(), root);
        browser.tab_shortcut(gdk::keys::constants::w, ctrl);
        wait(browser);
        assert_eq!(browser.tabs.borrow().len(), 1);
        assert_eq!(browser.current(), physics);
        browser.tab_shortcut(gdk::keys::constants::w, ctrl);
        assert_eq!(browser.tabs.borrow().len(), 1);
        eprintln!(
            "PASS tabs: folder context action, closing background/active tab, last tab retained"
        );

        browser.navigate(&root.join("invalid-pending-folder"), true, None, false);
        browser.save();
        assert_eq!(browser.prefs.borrow().data.tabs[0].path, physics);
        wait(browser);
        assert_eq!(browser.current(), physics);

        // A pending scan from another tab must never overwrite this one's view.
        let plus = browser
            .header
            .children()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.tooltip_text().as_deref() == Some("New tab · Ctrl+T"))
            .unwrap();
        plus.emit_clicked();
        wait(browser);
        assert_eq!(browser.current(), physics);
        browser.navigate(root, true, None, false);
        wait(browser);
        browser.columns[2].1.clicked();
        browser.notebook.set_current_page(Some(0));
        wait(browser);
        assert_eq!(browser.current(), physics);
        browser.notebook.set_current_page(Some(1));
        wait(browser);
        assert_eq!(browser.current(), root);
        assert_eq!(browser.notebook.current_page(), Some(1));
        browser.save();
        let saved = Preferences::load(prefs_path.into());
        assert_eq!(
            saved
                .data
                .tabs
                .iter()
                .map(|t| t.path.clone())
                .collect::<Vec<_>>(),
            vec![physics.to_path_buf(), root.to_path_buf()]
        );
        assert_eq!(saved.data.active_tab, 1);
        if let Some(path) = std::env::var_os("PERSONAL_FILES_TABS_SCREENSHOT") {
            screenshot(browser, &PathBuf::from(path));
        }
        browser.window.close();
        drain();
        let restored = Browser::new(app, None, Preferences::load(prefs_path.into()));
        wait(&restored);
        assert_eq!(restored.tabs.borrow().len(), 2);
        assert_eq!(restored.active_tab.get(), 1);
        assert_eq!(restored.notebook.current_page(), Some(1));
        assert_eq!(restored.current(), root);
        assert_eq!(restored.prefs.borrow().data.sort, Sort::Size);
        restored.notebook.set_current_page(Some(0));
        wait(&restored);
        assert_eq!(restored.current(), physics);
        assert_eq!(restored.prefs.borrow().data.sort, Sort::Name);
        eprintln!(
            "PASS tabs: rapid switching rejects stale scans; ordered tabs and active tab restore"
        );
        restored.new_tab(root.join("missing-folder"));
        wait(&restored);
        assert_eq!(restored.current(), core::home());
        restored.notebook.set_current_page(Some(0));
        wait(&restored);
        assert_eq!(restored.current(), physics);
        restored.window.close();
        drain();
        let mut missing = Preferences::load(prefs_path.into());
        missing.data.tabs[1].path = root.join("gone-after-restart");
        missing.data.active_tab = 1;
        missing.save().unwrap();
        let again = Browser::new(app, None, Preferences::load(prefs_path.into()));
        wait(&again);
        assert_eq!(again.current(), core::home());
        again.notebook.set_current_page(Some(0));
        wait(&again);
        assert_eq!(again.current(), physics);
        again.window.close();
        drain();
        eprintln!("PASS tabs: missing restored folder falls back without changing another tab");
    }

    #[test]
    #[ignore = "Needs a real graphical display or xvfb-run; uses only temporary fixture files"]
    fn desktop_story() {
        gtk::init().expect("GTK must connect to a graphical display");
        let app = gtk::Application::new(
            Some("io.monoclaw.PersonalFiles.Tests"),
            gio::ApplicationFlags::NON_UNIQUE,
        );
        app.register(None::<&gio::Cancellable>).unwrap();
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let physics = root.join("Library/long path/Physics");
        fs::create_dir_all(physics.join("Topics")).unwrap();
        fs::write(physics.join(".hidden"), "fixture").unwrap();
        let filename = "a b's $notes.txt";
        fs::write(physics.join(filename), "fixture").unwrap();
        let prefs_path = root.join("config/preferences.json");
        let browser = Browser::new(
            &app,
            Some(physics.clone()),
            Preferences::load(prefs_path.clone()),
        );
        wait(&browser);
        assert_eq!(browser.current(), physics);
        assert!(browser
            .displayed
            .borrow()
            .contains(&physics.join(".hidden")));
        eprintln!("PASS startup and hidden files");
        browser.address.set_text("Topics");
        browser.address.emit_activate();
        wait(&browser);
        assert_eq!(browser.current(), physics.join("Topics"));
        browser.address.set_text(&physics.to_string_lossy());
        browser.address.emit_activate();
        wait(&browser);
        eprintln!("PASS absolute and relative path entry");
        browser.pin.emit_clicked();
        assert!(browser.prefs.borrow().data.favorites.contains(&physics));
        browser.navigate(root, true, None, false);
        wait(&browser);
        let favorites = browser
            .sidebar
            .children()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::ListBox>().ok())
            .nth(1)
            .unwrap();
        let row = favorites.row_at_index(0).unwrap();
        favorites.emit_by_name::<()>("row-activated", &[&row]);
        wait(&browser);
        assert_eq!(browser.current(), physics);
        eprintln!("PASS favorite button and sidebar activation");
        select(&browser, filename);
        browser.copy(false, None);
        let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        assert_eq!(
            clipboard.wait_for_text().unwrap().as_str(),
            physics.join(filename).to_str().unwrap()
        );
        let text = browser.copy(true, None);
        assert_eq!(clipboard.wait_for_text().unwrap().as_str(), text);
        assert_eq!(
            text,
            core::shell_quote(physics.join(filename).to_str().unwrap())
        );
        eprintln!("PASS real clipboard plain and shell-quoted paths");
        let menu = browser.path_menu(vec![physics.clone()]);
        menu.children()[1]
            .clone()
            .downcast::<gtk::MenuItem>()
            .unwrap()
            .emit_activate();
        assert_eq!(
            clipboard.wait_for_text().unwrap().as_str(),
            physics.to_str().unwrap()
        );
        menu.detach();
        eprintln!("PASS context menu path copy");
        browser.columns[2].1.clicked();
        assert_eq!(browser.prefs.borrow().data.sort, Sort::Size);
        assert!(browser.prefs.borrow().data.descending);
        assert!(browser.displayed.borrow()[0].is_dir());
        browser.columns[2].1.clicked();
        assert!(!browser.prefs.borrow().data.descending);
        browser.columns[1].1.clicked();
        assert_eq!(browser.prefs.borrow().data.sort, Sort::Modified);
        eprintln!("PASS sort column signals and reversal");
        let row = select(&browser, "Topics");
        browser.tree.row_activated(&row, &browser.columns[0].1);
        wait(&browser);
        assert_eq!(browser.current(), physics.join("Topics"));
        browser.go_text(physics.join(".hidden").to_str().unwrap());
        wait(&browser);
        assert_eq!(browser.current(), physics);
        assert_eq!(browser.selected(), vec![physics.join(".hidden")]);
        eprintln!("PASS row activation and file-path selection");
        browser.navigate(&root.join("missing"), true, None, false);
        wait(&browser);
        assert_eq!(browser.current(), physics);
        assert!(browser.status.text().starts_with("Cannot open"));
        assert!(!browser
            .prefs
            .borrow()
            .data
            .visits
            .contains_key(&root.join("missing")));
        eprintln!("PASS failed navigation preserves current folder and history");
        browser.navigate(root, true, None, false);
        wait(&browser);
        browser.go_history(-1);
        wait(&browser);
        assert_eq!(browser.current(), physics);
        browser.go_history(1);
        wait(&browser);
        assert_eq!(browser.current(), root);
        let count = browser.prefs.borrow().data.visits[root].count;
        browser.refresh();
        wait(&browser);
        assert_eq!(browser.prefs.borrow().data.visits[root].count, count);
        eprintln!("PASS back, forward, refresh without inflated visits");
        browser.navigate(&physics, true, None, false);
        wait(&browser);
        select(&browser, "Topics");
        let uris = browser.drag_uris();
        assert_eq!(
            uris,
            vec![gio::File::for_path(physics.join("Topics"))
                .uri()
                .to_string()]
        );
        let dropped: Vec<_> = uris
            .iter()
            .filter_map(|u| gio::File::for_uri(u).path())
            .collect();
        assert_eq!(browser.add_favorites(&dropped), 1);
        assert!(browser
            .prefs
            .borrow()
            .data
            .favorites
            .contains(&physics.join("Topics")));
        eprintln!("PASS drag URI payload and favorite drop handling");
        for theme in ["silver", "midnight", "aurora", "auto"] {
            browser.set_theme(theme);
            assert!(!browser.status.text().contains("Appearance could not load"));
        }
        let image = root.join("background's image.png");
        let pixbuf =
            gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 32, 32).unwrap();
        pixbuf.fill(0x37679fff);
        pixbuf.savev(&image, "png", &[]).unwrap();
        browser.set_image(&image);
        assert_eq!(browser.prefs.borrow().data.image, image);
        assert!(!browser.status.text().contains("Appearance could not load"));
        browser.set_image(&root.join("missing.png"));
        assert_eq!(browser.prefs.borrow().data.image, image);
        browser.set_image(Path::new(""));
        assert!(browser.prefs.borrow().data.image.as_os_str().is_empty());
        eprintln!("PASS all GTK skins, image validation and removal");
        let count = browser.prefs.borrow().data.visits[&physics].count;
        fs::write(physics.join("new.txt"), "fixture").unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        while !browser
            .displayed
            .borrow()
            .contains(&physics.join("new.txt"))
            && Instant::now() < deadline
        {
            drain();
            thread::sleep(Duration::from_millis(20));
        }
        assert!(browser
            .displayed
            .borrow()
            .contains(&physics.join("new.txt")));
        assert_eq!(browser.prefs.borrow().data.visits[&physics].count, count);
        eprintln!("PASS real filesystem monitoring without inflated visits");
        let generation = browser.nav.borrow().generation;
        browser.finish(Scan {
            request: Request {
                generation: generation - 1,
                target: root.into(),
                select: None,
                record: true,
                history_index: None,
                fallback: false,
            },
            result: Ok(vec![]),
        });
        assert_eq!(browser.current(), physics);
        eprintln!("PASS stale background result rejected");
        browser.window.close();
        drain();
        drop(browser);
        let browser = Browser::new(&app, None, Preferences::load(prefs_path.clone()));
        wait(&browser);
        assert_eq!(browser.current(), physics);
        assert!(browser.prefs.borrow().data.favorites.contains(&physics));
        assert!(browser.prefs.borrow().recent(10).contains(&physics));
        eprintln!("PASS fresh-window restart persistence");
        browser.set_theme("silver");
        if let Some(path) = std::env::var_os("PERSONAL_FILES_SCREENSHOT") {
            screenshot(&browser, &PathBuf::from(path));
        }
        browser.prefs.borrow_mut().data.visits.clear();
        browser.rebuild_sidebar();
        browser.save();
        assert_eq!(browser.prefs.borrow().data.favorites.len(), 2);
        assert!(browser.prefs.borrow().data.visits.is_empty());
        eprintln!("PASS clearing visits retains favorites");
        sidebar_story(&browser, root, &physics);
        tabs_story(&app, &browser, root, &physics, &prefs_path);
        browser.window.close();
        drain();
    }
}
