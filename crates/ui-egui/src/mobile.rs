//! Phone and tablet layouts. The desktop arrangement (a menu bar and a dozen docked panels) cannot
//! be used on a narrow touch screen, so below [`TABLET_MAX`] points the web app draws its own
//! shell instead: a top bar with the menu drawer, the Program monitor, one panel at a time picked
//! from a bottom tab bar (phones), or the Program monitor beside one side panel over the Timeline
//! (tablets). The size is read every frame, so turning the device or resizing the window switches
//! layouts at once. Above the breakpoint, and in the desktop app, nothing changes.

use egui::{Color32, Rect, Sense, Stroke, pos2, vec2};
use serde_json::json;

use crate::FilmcraftApp;
use crate::dock::PanelKind;
use crate::icons::{self, Icon};
use crate::panels;
use crate::state::Mode;
use crate::theme::Tokens;

/// Windows narrower than this (in points, which are CSS pixels on the web) get the phone layout.
pub const PHONE_MAX: f32 = 700.0;
/// Windows narrower than this get the tablet layout; wider ones keep the desktop layout.
pub const TABLET_MAX: f32 = 1100.0;
/// Below this height (a phone held sideways) the phone layout is used up to [`TABLET_MAX`].
const SHORT_MAX: f32 = 500.0;
/// The smallest touch target, in points (Apple and Google both ask for 44).
pub const TOUCH: f32 = 44.0;
const TOP_BAR_H: f32 = 52.0;
const TAB_BAR_H: f32 = 60.0;
const SEGMENTS_H: f32 = TOUCH;

/// Which arrangement the window gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Phone,
    Tablet,
    Desktop,
}

impl Form {
    /// The arrangement for a window `size` points large.
    pub fn for_size(size: egui::Vec2) -> Form {
        let (w, h) = (size.x, size.y);
        if !(w.is_finite() && h.is_finite()) || w <= 0.0 || h <= 0.0 {
            return Form::Desktop;
        }
        if w < PHONE_MAX || (h < SHORT_MAX && w < TABLET_MAX) {
            Form::Phone
        } else if w < TABLET_MAX {
            Form::Tablet
        } else {
            Form::Desktop
        }
    }

    /// The arrangement for this frame. Only the web app adapts: the desktop app keeps its docked
    /// panels at every window size.
    pub fn current(ctx: &egui::Context) -> Form {
        if cfg!(target_arch = "wasm32") { Form::for_size(ctx.content_rect().size()) } else { Form::Desktop }
    }

    pub fn is_touch(self) -> bool {
        self != Form::Desktop
    }
}

/// Whether this frame uses a phone or tablet layout (touch-sized controls).
pub fn touch(ctx: &egui::Context) -> bool {
    Form::current(ctx).is_touch()
}

/// The bottom tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    Media,
    #[default]
    Timeline,
    Effects,
    Audio,
    Export,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Media => tl!("Media"),
            Tab::Timeline => tl!("Timeline"),
            Tab::Effects => tl!("Effects"),
            Tab::Audio => tl!("Audio"),
            Tab::Export => tl!("Export"),
        }
    }
    fn icon(self) -> Icon {
        match self {
            Tab::Media => Icon::Folder,
            Tab::Timeline => Icon::Sequence,
            Tab::Effects => Icon::Fx,
            Tab::Audio => Icon::Speaker,
            Tab::Export => Icon::Export,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
    /// The panels a tab switches between (a segmented control over them when more than one).
    pub fn panels(self) -> &'static [PanelKind] {
        use PanelKind::*;
        match self {
            Tab::Media => &[Project, MediaBrowser],
            Tab::Timeline => &[Timeline],
            Tab::Effects => &[Effects, EffectControls, Properties, LumetriColor],
            Tab::Audio => &[AudioTrackMixer, EssentialSound, AudioClipMixer],
            Tab::Export => &[],
        }
    }
}

/// The tabs of each layout: tablets always show the Timeline, so it has no tab there.
pub fn tabs(form: Form) -> &'static [Tab] {
    match form {
        Form::Tablet => &[Tab::Media, Tab::Effects, Tab::Audio, Tab::Export],
        _ => &[Tab::Media, Tab::Timeline, Tab::Effects, Tab::Audio, Tab::Export],
    }
}

/// What the phone and tablet layouts remember between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Shell {
    tab: Tab,
    /// The chosen panel of each tab's segmented control.
    sub: [u8; 5],
    /// A panel opened from the menu drawer (Window ▸ …), shown in place of the tab's panel.
    extra: Option<PanelKind>,
    menu_open: bool,
    /// The form last drawn, to restore the desktop style when leaving touch layouts.
    touch_style: bool,
}

fn shell_id() -> egui::Id {
    egui::Id::unique("vl-responsive-shell")
}

fn load(ctx: &egui::Context) -> Shell {
    ctx.data(|d| d.get_temp::<Shell>(shell_id())).unwrap_or_default()
}

fn store(ctx: &egui::Context, shell: Shell) {
    ctx.data_mut(|d| d.insert_temp(shell_id(), shell));
}

/// The desktop values of the style fields touch layouts change.
#[derive(Clone, Copy, Debug, PartialEq)]
struct DesktopStyle {
    interact: egui::Vec2,
    button_padding: egui::Vec2,
    item_spacing: egui::Vec2,
    icon_width: f32,
    icon_spacing: f32,
    bar_width: f32,
}

impl DesktopStyle {
    fn of(style: &egui::Style) -> Self {
        let s = &style.spacing;
        DesktopStyle {
            interact: s.interact_size,
            button_padding: s.button_padding,
            item_spacing: s.item_spacing,
            icon_width: s.icon_width,
            icon_spacing: s.icon_spacing,
            bar_width: s.scroll.bar_width,
        }
    }
    fn touch() -> Self {
        DesktopStyle {
            interact: vec2(44.0, 34.0),
            button_padding: vec2(10.0, 7.0),
            item_spacing: vec2(8.0, 6.0),
            icon_width: 20.0,
            icon_spacing: 8.0,
            bar_width: 10.0,
        }
    }
    fn apply(self, style: &mut egui::Style) {
        let s = &mut style.spacing;
        s.interact_size = self.interact;
        s.button_padding = self.button_padding;
        s.item_spacing = self.item_spacing;
        s.icon_width = self.icon_width;
        s.icon_width_inner = self.icon_width * 0.6;
        s.icon_spacing = self.icon_spacing;
        s.scroll.bar_width = self.bar_width;
    }
}

fn desktop_style_id() -> egui::Id {
    egui::Id::unique("vl-responsive-desktop-style")
}

/// Larger buttons, rows and checkboxes everywhere (menus and popups included) while a touch layout
/// is shown; the desktop values come back when the window grows past the breakpoint again.
pub fn apply_touch_style(ctx: &egui::Context, form: Form) {
    let mut shell = load(ctx);
    let touch = DesktopStyle::touch();
    if form.is_touch() {
        let current = DesktopStyle::of(&ctx.global_style());
        if current != touch {
            // a theme change may have rewritten the style: what it wrote is the desktop style
            ctx.data_mut(|d| d.insert_temp(desktop_style_id(), current));
            ctx.all_styles_mut(|s| touch.apply(s));
        }
        shell.touch_style = true;
    } else if shell.touch_style {
        if let Some(desktop) = ctx.data(|d| d.get_temp::<DesktopStyle>(desktop_style_id())) {
            ctx.all_styles_mut(|s| desktop.apply(s));
        }
        shell.touch_style = false;
    } else {
        return;
    }
    store(ctx, shell);
}

/// Draws the whole window in a phone or tablet layout.
pub(crate) fn show(app: &mut FilmcraftApp, ui: &mut egui::Ui, full: Rect, form: Form) {
    let ctx = ui.ctx().clone();
    let t = app.tokens;
    let mut shell = load(&ctx);
    // the mode can change from elsewhere (a menu command, Export's own buttons)
    match app.ui.mode {
        Mode::Export => shell.tab = Tab::Export,
        _ if shell.tab == Tab::Export => shell.tab = if form == Form::Tablet { Tab::Media } else { Tab::Timeline },
        _ => {}
    }
    if form == Form::Tablet && shell.tab == Tab::Timeline {
        shell.tab = Tab::Media;
    }
    ui.painter().rect_filled(full, 0.0, t.app_bg);
    let top = Rect::from_min_size(full.min, vec2(full.width(), TOP_BAR_H));
    let bar = Rect::from_min_max(pos2(full.min.x, full.max.y - TAB_BAR_H), full.max);
    let body = Rect::from_min_max(pos2(full.min.x, top.max.y), pos2(full.max.x, bar.min.y));
    top_bar(app, ui, top, &mut shell);
    tab_bar(app, ui, bar, form, &mut shell);
    let mut bodies = Vec::new();
    match app.ui.mode {
        Mode::Import => panels::import_mode::show(app, ui, body),
        Mode::Export => panels::export_mode::show(app, ui, body),
        Mode::Edit => match form {
            Form::Tablet => tablet(app, ui, body, &mut shell, &mut bodies),
            _ => phone(app, ui, body, &mut shell, &mut bodies),
        },
    }
    let layer = ui.layer_id();
    ctx.data_mut(|d| d.insert_temp(crate::dock_ui::bodies_id(), bodies.into_iter().map(|(p, r)| (p, r, layer)).collect::<Vec<_>>()));
    toast(app, ui, Rect::from_min_max(body.min, pos2(body.max.x, body.max.y - 8.0)));
    if shell.menu_open {
        drawer(app, &ctx, full, &mut shell);
    }
    store(&ctx, shell);
}

/// Hamburger, the app and project name, Undo and Redo.
fn top_bar(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, shell: &mut Shell) {
    let t = app.tokens;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.header_bg);
    p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, Color32::BLACK));
    let menu = Rect::from_min_size(pos2(rect.min.x + 4.0, rect.center().y - TOUCH / 2.0), vec2(TOUCH, TOUCH));
    if icon_button(app, ui, menu, Icon::Hamburger, "mobile.menu", tl!("Menu"), shell.menu_open).clicked() {
        shell.menu_open = !shell.menu_open;
    }
    let mut right = rect.max.x - 4.0;
    for (icon, id, label, command) in [(Icon::Redo, "mobile.redo", tl!("Redo"), "edit.redo"), (Icon::Undo, "mobile.undo", tl!("Undo"), "edit.undo")] {
        let r = Rect::from_min_size(pos2(right - TOUCH, rect.center().y - TOUCH / 2.0), vec2(TOUCH, TOUCH));
        right = r.min.x;
        if icon_button(app, ui, r, icon, id, label, false).clicked() {
            let ctx = ui.ctx().clone();
            if let Err(e) = crate::menus::invoke(app, &ctx, command, json!({})) {
                app.ui.status = e;
            }
        }
    }
    // "VL Films" over the project name, both cut short with … when the bar is narrow
    let left = menu.max.x + 8.0;
    let width = (right - 8.0 - left).max(0.0);
    let brand = p.layout(String::from("VL Films"), Tokens::ui(15.0), t.accent, f32::INFINITY);
    let name = if app.session.is_dirty() { format!("{} •", app.session.project.name) } else { app.session.project.name.clone() };
    let mut job = egui::text::LayoutJob::simple_singleline(name, Tokens::ui(12.0), t.text_dim);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    let project = p.layout_job(job);
    let clip = p.with_clip_rect(Rect::from_min_max(pos2(left, rect.min.y), pos2(left + width, rect.max.y)));
    let total = brand.size().y + project.size().y;
    let y = rect.center().y - total / 2.0;
    let brand_h = brand.size().y;
    clip.galley(pos2(left, y), brand, t.accent);
    clip.galley(pos2(left, y + brand_h), project, t.text_dim);
}

fn icon_button(app: &mut FilmcraftApp, ui: &mut egui::Ui, r: Rect, icon: Icon, id: &str, label: &str, on: bool) -> egui::Response {
    let t = app.tokens;
    let resp = ui.interact(r, egui::Id::unique(("vl-touch-button", id)), Sense::click()).on_hover_text(label);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    app.auto.add(id, r, label);
    if on || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(r.shrink(4.0), 8.0, t.pressed);
    } else if resp.hovered() {
        ui.painter().rect_filled(r.shrink(4.0), 8.0, t.hover);
    }
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(22.0, 22.0)), icon, if on { t.accent } else { t.text });
    resp
}

/// Text that fits `width`: the largest size from `size` down to 9 points, cut short with … below that.
fn fitted(painter: &egui::Painter, text: &str, size: f32, width: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    let mut s = size;
    while s > 9.0 {
        let g = painter.layout_no_wrap(text.to_owned(), Tokens::ui(s), color);
        if g.size().x <= width {
            return g;
        }
        s -= 0.5;
    }
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), Tokens::ui(9.0), color);
    job.wrap.max_width = width.max(1.0);
    job.wrap.max_rows = 1;
    painter.layout_job(job)
}

/// The bottom tab bar: an icon over a label per tab, the whole cell a touch target.
fn tab_bar(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, form: Form, shell: &mut Shell) {
    let t = app.tokens;
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.header_bg);
    p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, Color32::BLACK));
    let tabs = tabs(form);
    let w = rect.width() / tabs.len() as f32;
    for (i, tab) in tabs.iter().copied().enumerate() {
        let cell = Rect::from_min_size(pos2(rect.min.x + i as f32 * w, rect.min.y), vec2(w, rect.height()));
        let label = tab.label();
        let resp = ui.interact(cell, egui::Id::unique(("vl-tab", i)), Sense::click());
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, shell.tab == tab, label));
        app.auto.add(&format!("mobile.tab.{tab:?}"), cell, label);
        let active = shell.tab == tab && shell.extra.is_none();
        let color = if active { t.accent } else if resp.hovered() { t.text } else { t.text_dim };
        if active {
            p.rect_filled(Rect::from_center_size(pos2(cell.center().x, cell.min.y + 2.0), vec2((w * 0.5).min(48.0), 3.0)), 1.5, t.accent);
        }
        icons::paint(&p, Rect::from_center_size(pos2(cell.center().x, cell.min.y + 21.0), vec2(22.0, 22.0)), tab.icon(), color);
        let g = fitted(&p, label, 11.5, w - 6.0, color);
        p.galley(pos2(cell.center().x - g.size().x / 2.0, cell.min.y + 37.0), g, color);
        if resp.clicked() {
            shell.tab = tab;
            shell.extra = None;
            shell.menu_open = false;
            app.ui.mode = if tab == Tab::Export { Mode::Export } else { Mode::Edit };
            if let Some(panel) = tab.panels().get(shell.sub[tab.index()] as usize) {
                app.ui.focused = *panel;
            }
        }
    }
}

/// A segmented control over the panels of `tab`, `rect` tall [`SEGMENTS_H`]; returns the panel shown.
fn segments(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, tab: Tab, shell: &mut Shell) -> PanelKind {
    let t = app.tokens;
    let panels = tab.panels();
    let chosen = (shell.sub[tab.index()] as usize).min(panels.len().saturating_sub(1));
    let p = ui.painter().clone();
    p.rect_filled(rect, 0.0, t.tab_bg);
    let inner = rect.shrink2(vec2(8.0, 5.0));
    p.rect_filled(inner, 8.0, t.field_bg);
    let w = inner.width() / panels.len() as f32;
    for (i, panel) in panels.iter().copied().enumerate() {
        let cell = Rect::from_min_size(pos2(inner.min.x + i as f32 * w, inner.min.y), vec2(w, inner.height()));
        let label = crate::i18n::t(panel.title());
        let resp = ui.interact(cell, egui::Id::unique(("vl-segment", tab.index(), i)), Sense::click());
        app.auto.add(&format!("mobile.panel.{}", panel.id()), cell, label);
        let active = i == chosen;
        if active {
            p.rect_filled(cell.shrink(2.0), 6.0, t.pill_active_bg);
        }
        let color = if active { t.tab_text_active } else { t.tab_text };
        let g = fitted(&p, label, 13.0, w - 10.0, color);
        p.galley(cell.center() - g.size() / 2.0, g, color);
        if resp.clicked() {
            shell.sub[tab.index()] = i as u8;
            app.ui.focused = panel;
        }
    }
    panels[chosen]
}

/// One panel body in `rect`, as the dock area draws it: tapping it gives it the focus.
fn panel(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, kind: PanelKind, bodies: &mut Vec<(PanelKind, Rect)>) {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    let t = app.tokens;
    ui.painter().rect_filled(rect, 0.0, t.panel_bg);
    if ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))) {
        app.ui.focused = kind;
    }
    bodies.push((kind, rect));
    app.auto.add(&format!("panel.{}", kind.id()), rect, kind.title());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).id_salt(("panel", kind.id())));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    panels::show(app, &mut child, kind, rect);
}

/// The Timeline with the Tools above it and zoom buttons over its corner.
fn timeline(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, bodies: &mut Vec<(PanelKind, Rect)>) {
    let t = app.tokens;
    let tools_h = panels::tools::strip_extent(rect.width(), TOUCH).min(rect.height() * 0.4);
    let tools = Rect::from_min_size(rect.min, vec2(rect.width(), tools_h));
    ui.painter().rect_filled(tools, 0.0, t.panel_bg);
    ui.painter().line_segment([tools.left_bottom(), tools.right_bottom()], Stroke::new(1.0, t.separator));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(tools).id_salt("mobile-tools"));
    child.set_clip_rect(tools.intersect(ui.clip_rect()));
    panels::tools::show_sized(app, &mut child, tools, TOUCH);
    let line = Rect::from_min_max(pos2(rect.min.x, tools.max.y + 1.0), rect.max);
    // On a phone the track headers give up some width to the clips: the name column narrows
    // (the patch, lock and target buttons stay), unless the user drags the column wider here.
    let saved = app.ui.timeline.header_w;
    let narrow = (line.width() * 0.34).clamp(116.0, 204.0);
    if saved > narrow {
        app.ui.timeline.header_w = narrow;
    }
    panel(app, ui, line, PanelKind::Timeline, bodies);
    if saved > narrow && app.ui.timeline.header_w == narrow {
        app.ui.timeline.header_w = saved;
    }
    // −, fit and + over the bottom right corner, clear of the track headers on the left
    let mut x = line.max.x - 8.0;
    let y = line.max.y - 8.0 - TOUCH;
    if line.height() < TOUCH * 2.0 {
        return;
    }
    for (icon, id, label, command) in [
        (Icon::Plus, "mobile.zoomIn", tl!("Zoom In"), "view.zoomIn"),
        (Icon::Fullscreen, "mobile.zoomFit", tl!("Zoom to Sequence"), "view.zoomToSequence"),
        (Icon::Zoom, "mobile.zoomOut", tl!("Zoom Out"), "view.zoomOut"),
    ] {
        let r = Rect::from_min_size(pos2(x - TOUCH, y), vec2(TOUCH, TOUCH));
        x = r.min.x - 6.0;
        let resp = ui.interact(r, egui::Id::unique(("vl-zoom", id)), Sense::click()).on_hover_text(label);
        app.auto.add(id, r, label);
        let fill = if resp.is_pointer_button_down_on() { t.pressed } else { t.header_bg.gamma_multiply(0.92) };
        ui.painter().circle(r.center(), TOUCH / 2.0 - 2.0, fill, Stroke::new(1.0, t.separator));
        if id == "mobile.zoomOut" {
            // a minus sign: the icon set has a plus but no minus
            let c = r.center();
            ui.painter().line_segment([c - vec2(7.0, 0.0), c + vec2(7.0, 0.0)], Stroke::new(2.0, t.text));
        } else {
            icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(18.0, 18.0)), icon, t.text);
        }
        if resp.clicked() {
            let ctx = ui.ctx().clone();
            if let Err(e) = crate::menus::invoke(app, &ctx, command, json!({})) {
                app.ui.status = e;
            }
        }
    }
}

/// A panel opened from the menu drawer, under a bar with its name and a close button.
fn extra(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, kind: PanelKind, shell: &mut Shell, bodies: &mut Vec<(PanelKind, Rect)>) {
    let t = app.tokens;
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), SEGMENTS_H));
    ui.painter().rect_filled(head, 0.0, t.tab_bg);
    let g = fitted(ui.painter(), crate::i18n::t(kind.title()), 14.0, head.width() - TOUCH - 24.0, t.tab_text_active);
    ui.painter().galley(pos2(head.min.x + 12.0, head.center().y - g.size().y / 2.0), g, t.tab_text_active);
    let close = Rect::from_min_size(pos2(head.max.x - TOUCH, head.min.y), vec2(TOUCH, TOUCH));
    if icon_button(app, ui, close, Icon::Close, "mobile.extra.close", tl!("Close Panel"), false).clicked() {
        shell.extra = None;
    }
    panel(app, ui, Rect::from_min_max(pos2(rect.min.x, head.max.y), rect.max), kind, bodies);
}

/// What a tab shows in `rect`: the Timeline (with its tools), or the segmented control and a panel.
fn tab_content(app: &mut FilmcraftApp, ui: &mut egui::Ui, rect: Rect, tab: Tab, shell: &mut Shell, bodies: &mut Vec<(PanelKind, Rect)>) {
    if let Some(kind) = shell.extra {
        return extra(app, ui, rect, kind, shell, bodies);
    }
    if tab == Tab::Timeline {
        return timeline(app, ui, rect, bodies);
    }
    let panels = tab.panels();
    if panels.len() > 1 {
        let seg = Rect::from_min_size(rect.min, vec2(rect.width(), SEGMENTS_H));
        let kind = segments(app, ui, seg, tab, shell);
        panel(app, ui, Rect::from_min_max(pos2(rect.min.x, seg.max.y), rect.max), kind, bodies);
    } else if let Some(kind) = panels.first() {
        panel(app, ui, rect, *kind, bodies);
    }
}

/// The Program monitor's share of a phone held upright: a 16:9 picture with its transport
/// controls, within limits.
fn program_height(width: f32, height: f32) -> f32 {
    (width * 9.0 / 16.0 + 76.0).clamp(160.0_f32.min(height * 0.45), (height * 0.45).max(120.0))
}

/// Phone: the Program monitor over the tab's panel (beside it when held sideways).
fn phone(app: &mut FilmcraftApp, ui: &mut egui::Ui, body: Rect, shell: &mut Shell, bodies: &mut Vec<(PanelKind, Rect)>) {
    let landscape = body.width() > body.height() * 1.2;
    let (program, rest) = if landscape {
        let x = body.min.x + (body.width() * 0.48).round();
        (Rect::from_min_max(body.min, pos2(x, body.max.y)), Rect::from_min_max(pos2(x + 1.0, body.min.y), body.max))
    } else {
        let y = body.min.y + program_height(body.width(), body.height()).round();
        (Rect::from_min_max(body.min, pos2(body.max.x, y)), Rect::from_min_max(pos2(body.min.x, y + 1.0), body.max))
    };
    panel(app, ui, program, PanelKind::Program, bodies);
    let tab = shell.tab;
    tab_content(app, ui, rest, tab, shell, bodies);
}

/// Tablet: the Program monitor beside the tab's panel, over the Timeline and its tools.
fn tablet(app: &mut FilmcraftApp, ui: &mut egui::Ui, body: Rect, shell: &mut Shell, bodies: &mut Vec<(PanelKind, Rect)>) {
    let portrait = body.height() > body.width();
    let top_h = (body.height() * if portrait { 0.5 } else { 0.56 }).round();
    let top = Rect::from_min_size(body.min, vec2(body.width(), top_h));
    let split = top.min.x + (top.width() * if portrait { 0.56 } else { 0.6 }).round();
    let program = Rect::from_min_max(top.min, pos2(split, top.max.y));
    let side = Rect::from_min_max(pos2(split + 1.0, top.min.y), top.max);
    panel(app, ui, program, PanelKind::Program, bodies);
    let tab = shell.tab;
    tab_content(app, ui, side, tab, shell, bodies);
    let bottom = Rect::from_min_max(pos2(body.min.x, top.max.y + 1.0), body.max);
    timeline(app, ui, bottom, bodies);
}

/// The status message as a toast over the bottom of the content, tap to dismiss.
fn toast(app: &mut FilmcraftApp, ui: &mut egui::Ui, area: Rect) {
    if app.ui.status.is_empty() {
        return;
    }
    let t = app.tokens;
    let now = ui.input(|i| i.time);
    if app.ui.status != app.status_seen.0 {
        app.status_seen = (app.ui.status.clone(), now);
    } else if now - app.status_seen.1 > 6.0 {
        app.ui.status.clear();
        return;
    } else {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    let width = (area.width() - 24.0).min(520.0);
    let mut job = egui::text::LayoutJob::simple(app.ui.status.clone(), Tokens::ui(13.0), t.text, width - 24.0);
    job.wrap.max_rows = 3;
    let g = ui.painter().layout_job(job);
    let r = Rect::from_min_size(pos2(area.center().x - width / 2.0, area.max.y - g.size().y - 20.0), vec2(width, g.size().y + 20.0));
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("vl-toast")));
    painter.rect(r, 10.0, t.header_bg, Stroke::new(1.0, t.separator), egui::StrokeKind::Inside);
    painter.galley(pos2(r.min.x + 12.0, r.min.y + 10.0), g, t.text);
    app.auto.add("status.text", r, &app.ui.status.clone());
    if ui.input(|i| i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|p| r.contains(p))) {
        app.ui.status.clear();
    }
}

/// The menu drawer: the modes, every menu of the menu bar, and the panels the tabs don't hold.
fn drawer(app: &mut FilmcraftApp, ctx: &egui::Context, full: Rect, shell: &mut Shell) {
    let t = app.tokens;
    let width = (full.width() * 0.86).min(360.0);
    // the dimmed rest of the window closes the drawer
    let backdrop = egui::Area::new(egui::Id::unique("vl-drawer-backdrop")).order(egui::Order::Middle).fixed_pos(full.min).show(ctx, |ui| {
        let resp = ui.allocate_rect(full, Sense::click());
        ui.painter().rect_filled(full, 0.0, Color32::from_black_alpha(140));
        resp
    });
    if backdrop.inner.clicked() {
        shell.menu_open = false;
        return;
    }
    let mut chosen: Option<String> = None;
    let mut close = false;
    egui::Area::new(egui::Id::unique("vl-drawer")).order(egui::Order::Foreground).fixed_pos(full.min).show(ctx, |ui| {
        let rect = Rect::from_min_size(full.min, vec2(width, full.height()));
        ui.painter().rect_filled(rect, 0.0, t.panel_bg);
        ui.painter().line_segment([rect.right_top(), rect.right_bottom()], Stroke::new(1.0, Color32::BLACK));
        ui.set_min_size(rect.size());
        ui.set_max_size(rect.size());
        let head = Rect::from_min_size(rect.min, vec2(width, TOP_BAR_H));
        ui.painter().rect_filled(head, 0.0, t.header_bg);
        let g = ui.painter().layout_no_wrap("VL Films".into(), Tokens::ui(17.0), t.accent);
        ui.painter().galley(pos2(head.min.x + 16.0, head.center().y - g.size().y / 2.0), g, t.accent);
        let x = Rect::from_min_size(pos2(head.max.x - TOUCH - 4.0, head.center().y - TOUCH / 2.0), vec2(TOUCH, TOUCH));
        if icon_button(app, ui, x, Icon::Close, "mobile.menu.close", tl!("Close"), false).clicked() {
            close = true;
        }
        let list = Rect::from_min_max(pos2(rect.min.x, head.max.y), rect.max);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink2(vec2(10.0, 8.0))).layout(egui::Layout::top_down_justified(egui::Align::Min)));
        child.style_mut().spacing.button_padding = vec2(12.0, 11.0);
        child.style_mut().spacing.interact_size.y = TOUCH;
        child.style_mut().spacing.item_spacing.y = 2.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut child, |ui| {
            // the three modes
            ui.horizontal(|ui| {
                let w = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
                for (mode, label) in [(Mode::Import, tl!("Import")), (Mode::Edit, tl!("Edit")), (Mode::Export, tl!("Export"))] {
                    let b = egui::Button::selectable(app.ui.mode == mode, label).min_size(vec2(w, TOUCH));
                    let r = ui.add(b);
                    app.auto.add(&format!("mobile.mode.{mode:?}"), r.rect, label);
                    if r.clicked() {
                        app.ui.mode = mode;
                        close = true;
                    }
                }
            });
            ui.separator();
            chosen = crate::menus::menu_sheet(app, ui);
            ui.separator();
            egui::CollapsingHeader::new(egui::RichText::new(tl!("Panels")).strong()).id_salt("vl-drawer-panels").show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    for kind in PanelKind::ALL {
                        let r = ui.selectable_label(shell.extra == Some(kind), crate::i18n::t(kind.title()));
                        app.auto.add(&format!("mobile.open.{}", kind.id()), r.rect, kind.title());
                        if r.clicked() {
                            open_panel(app, shell, kind);
                            close = true;
                        }
                    }
                });
            });
            ui.add_space(8.0);
            let r = ui.button(format!("🌐  {}", tl!("Visit Visualab Studios (visualabstudios.com)")));
            if r.clicked() {
                crate::links::open(app, ui.ctx(), crate::links::DISCORD);
                close = true;
            }
            ui.add_space(24.0);
        });
    });
    if let Some(id) = chosen {
        close = true;
        let panel = id.strip_prefix("window.panel.").and_then(PanelKind::from_name);
        match panel {
            // Window ▸ a panel: shown in place of the tab's panel, there is no room to dock it
            Some(kind) => open_panel(app, shell, kind),
            None => {
                if let Err(e) = crate::menus::invoke(app, ctx, &id, json!({})) {
                    app.ui.status = e;
                }
            }
        }
    }
    if close {
        shell.menu_open = false;
    }
}

/// Shows `kind`: on its own tab when one holds it, else in place of the current tab's panel.
fn open_panel(app: &mut FilmcraftApp, shell: &mut Shell, kind: PanelKind) {
    app.ui.mode = Mode::Edit;
    app.ui.focused = kind;
    shell.extra = None;
    if kind == PanelKind::Program {
        return;
    }
    for tab in [Tab::Media, Tab::Timeline, Tab::Effects, Tab::Audio] {
        if let Some(i) = tab.panels().iter().position(|p| *p == kind) {
            shell.tab = tab;
            shell.sub[tab.index()] = i as u8;
            return;
        }
    }
    shell.extra = Some(kind);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints_follow_the_window_size() {
        assert_eq!(Form::for_size(vec2(390.0, 844.0)), Form::Phone);
        assert_eq!(Form::for_size(vec2(844.0, 390.0)), Form::Phone);
        assert_eq!(Form::for_size(vec2(820.0, 1180.0)), Form::Tablet);
        assert_eq!(Form::for_size(vec2(1024.0, 768.0)), Form::Tablet);
        assert_eq!(Form::for_size(vec2(1180.0, 820.0)), Form::Desktop);
        assert_eq!(Form::for_size(vec2(1440.0, 900.0)), Form::Desktop);
        assert_eq!(Form::for_size(vec2(0.0, 0.0)), Form::Desktop);
        assert_eq!(Form::for_size(vec2(f32::NAN, 800.0)), Form::Desktop);
    }

    #[test]
    fn the_desktop_app_keeps_its_layout() {
        let ctx = egui::Context::default();
        if !cfg!(target_arch = "wasm32") {
            assert_eq!(Form::current(&ctx), Form::Desktop);
        }
    }

    #[test]
    fn tabs_hold_distinct_panels() {
        let mut seen = Vec::new();
        for tab in tabs(Form::Phone) {
            for p in tab.panels() {
                assert!(!seen.contains(p), "{p:?} is on two tabs");
                seen.push(*p);
            }
        }
        assert!(!seen.contains(&PanelKind::Program), "the Program monitor is always shown");
        assert!(!tabs(Form::Tablet).contains(&Tab::Timeline), "tablets always show the Timeline");
    }

    #[test]
    fn the_program_monitor_leaves_room_below_it() {
        for (w, h) in [(390.0, 680.0), (320.0, 480.0), (430.0, 800.0)] {
            let p = program_height(w, h);
            assert!(p <= h * 0.45 + 0.5 && p >= 120.0_f32.min(h * 0.45), "{w}x{h}: {p}");
        }
    }
}
