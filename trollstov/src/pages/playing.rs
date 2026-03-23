use audio::AudioRating;
use database::Database;
use jukebox::Jukebox;
use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
    widgets::{Block, Padding},
};
use ratatui_image::StatefulImage;
use widgets::{List, ListItem, Shortcut, Shortcuts};

use crate::{
    app::{Action, FrontCover, FrontCoverStatus, ScreenSize},
    pages::Route,
    settings::Colors,
    symbols,
};

pub struct PlayingPage {
    current_qi: Option<usize>,
    list: List,
    view_mode: ViewMode,
}

#[derive(Clone, Copy)]
enum ViewMode {
    Queue,
    Cover,
    Both,
}

impl ViewMode {
    const fn next(self, screen_size: ScreenSize) -> Self {
        match screen_size {
            ScreenSize::Small => match self {
                ViewMode::Queue => ViewMode::Cover,
                ViewMode::Cover | ViewMode::Both => ViewMode::Queue,
            },
            ScreenSize::Medium | ScreenSize::Large => match self {
                ViewMode::Queue => ViewMode::Cover,
                ViewMode::Cover => ViewMode::Both,
                ViewMode::Both => ViewMode::Queue,
            },
        }
    }
}

impl PlayingPage {
    pub const fn new() -> Self {
        Self {
            current_qi: None,
            list: List::new(),
            view_mode: ViewMode::Both,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        screen_size: ScreenSize,
        front_cover: &mut FrontCover,
        front_cover_status: FrontCoverStatus,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        self.update_scroll_on_new_track(jb);

        // Render based on screen size and view mode
        match screen_size {
            ScreenSize::Small => match self.view_mode {
                ViewMode::Queue => {
                    self.render_queue(area, buf, db, jb, colors);
                }
                ViewMode::Cover => {
                    render_cover_with_stars(
                        area,
                        buf,
                        db,
                        jb,
                        front_cover,
                        front_cover_status,
                        colors,
                    );
                }
                ViewMode::Both => {
                    self.view_mode = ViewMode::Cover;
                    render_cover_with_stars(
                        area,
                        buf,
                        db,
                        jb,
                        front_cover,
                        front_cover_status,
                        colors,
                    );
                }
            },
            ScreenSize::Medium | ScreenSize::Large => {
                match self.view_mode {
                    ViewMode::Queue => {
                        self.render_queue(area, buf, db, jb, colors);
                    }
                    ViewMode::Cover => {
                        render_cover_with_stars(
                            area,
                            buf,
                            db,
                            jb,
                            front_cover,
                            front_cover_status,
                            colors,
                        );
                    }
                    ViewMode::Both => {
                        let [cover_area, _, queue_area] = Layout::horizontal([
                            Constraint::Percentage(40),
                            Constraint::Length(1),
                            Constraint::Min(3),
                        ])
                        .areas(area);

                        render_cover_with_stars(
                            cover_area,
                            buf,
                            db,
                            jb,
                            front_cover,
                            front_cover_status,
                            colors,
                        );
                        self.render_queue(queue_area, buf, db, jb, colors);
                    }
                }

                // Shortcuts
                if !jb.is_empty() {
                    match self.view_mode {
                        ViewMode::Queue => {
                            shortcuts.extend([
                                Shortcut::new("Play", symbols::ENTER),
                                Shortcut::new("Move", symbols::shift!("m")),
                                Shortcut::new("Shuffle", "s"),
                                Shortcut::new("Remove", "r"),
                                Shortcut::new("Clear", "c"),
                                Shortcut::new("Goto", "g"),
                            ]);
                        }
                        ViewMode::Cover => {
                            if jb.has_current() {
                                shortcuts.extend([
                                    Shortcut::new("Rating", "0-5"),
                                    Shortcut::new("Goto", "g"),
                                ]);
                            }
                        }
                        ViewMode::Both => {
                            shortcuts.push(Shortcut::new("Play", symbols::ENTER));
                            if jb.has_current() {
                                shortcuts.push(Shortcut::new("Rating", "0-5"));
                            }
                            shortcuts.extend([
                                Shortcut::new("Move", symbols::shift!("m")),
                                Shortcut::new("Shuffle", "s"),
                                Shortcut::new("Remove", "r"),
                                Shortcut::new("Clear", "c"),
                                Shortcut::new("Goto", "g"),
                            ]);
                        }
                    }
                }

                shortcuts.push(Shortcut::new("View", "v"));
            }
        }
    }

    pub fn on_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
        db: &mut Database,
        jb: &mut Jukebox,
        screen_size: ScreenSize,
    ) -> Action {
        if jb.is_empty() {
            if let KeyCode::Char('v') = key {
                self.view_mode = self.view_mode.next(screen_size);
                return Action::Render;
            }

            return Action::None;
        }

        match key {
            KeyCode::Enter => {
                if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                    let index = self.list.index();
                    jb.play_index(index, db);
                }
            }
            KeyCode::Char(c) => match c {
                '0' | '1' | '2' | '3' | '4' | '5' => {
                    if matches!(self.view_mode, ViewMode::Cover | ViewMode::Both) {
                        if let Some(id) = jb.current_track_id() {
                            let rating = AudioRating::from_char(c).unwrap();
                            db.write_rating(id, rating);
                        }
                    }
                }
                'c' => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        if jb.clear() {
                            return Action::Render;
                        }
                    }
                }
                's' => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        if jb.shuffle() {
                            return Action::Render;
                        }
                    }
                }
                'g' => match self.view_mode {
                    ViewMode::Queue | ViewMode::Both => {
                        let index = self.list.index();
                        let id = jb.get(index);
                        return Action::Route(Route::Tracks(id));
                    }
                    ViewMode::Cover => {
                        if let Some(id) = jb.current_track_id() {
                            return Action::Route(Route::Tracks(Some(id)));
                        }
                    }
                },
                'm' => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        let (start, end) = {
                            let selection = self.list.selection_inclusive();
                            (*selection.start(), *selection.end())
                        };

                        let success = if start == end {
                            jb.move_down(start)
                        } else {
                            jb.move_down_range(start, end)
                        };

                        if success {
                            self.current_qi = jb.current_queue_index();
                            self.list.move_selection_down();
                            return Action::Render;
                        }
                    }
                }
                'M' => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        let (start, end) = {
                            let selection = self.list.selection_inclusive();
                            (*selection.start(), *selection.end())
                        };

                        let success = if start == end {
                            jb.move_up(start)
                        } else {
                            jb.move_up_range(start, end)
                        };

                        if success {
                            self.current_qi = jb.current_queue_index();
                            self.list.move_selection_up();
                            return Action::Render;
                        }
                    }
                }
                'r' => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        let (start, end) = {
                            let selection = self.list.selection_inclusive();
                            (*selection.start(), *selection.end())
                        };

                        let success = if start == end {
                            jb.remove(start)
                        } else {
                            jb.remove_range(start, end)
                        };

                        if success {
                            self.current_qi = jb.current_queue_index();
                            self.list.set_index(start).set_selector(None);
                            return Action::Render;
                        }
                    }
                }
                'v' => {
                    self.view_mode = self.view_mode.next(screen_size);
                    return Action::Render;
                }
                _ => {
                    if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                        if self.list.input(key, _modifiers) {
                            return Action::Render;
                        }
                    }
                }
            },
            _ => {
                if matches!(self.view_mode, ViewMode::Queue | ViewMode::Both) {
                    if self.list.input(key, _modifiers) {
                        return Action::Render;
                    }
                }
            }
        }

        Action::None
    }

    pub fn on_exit(&mut self) {}

    fn update_scroll_on_new_track(&mut self, jb: &Jukebox) {
        let current_queue_index = jb.current_queue_index();
        if self.current_qi != current_queue_index {
            self.current_qi = current_queue_index;
            if let Some(idx) = current_queue_index {
                self.list.set_index(idx).set_selector(None);
            }
        }
    }

    fn render_queue(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        db: &Database,
        jb: &Jukebox,
        colors: &Colors,
    ) {
        let block = Block::bordered()
            .border_style(colors.neutral)
            .padding(Padding::horizontal(1));
        let queue_inner_area = block.inner(area);
        block.render(area, buf);

        // Title for bordered play queue
        utils::format_int2(jb.history(), jb.queue(), |hlen, qlen| {
            widgets::print_asciis(
                Rect {
                    y: area.y,
                    height: 1,
                    ..queue_inner_area
                },
                buf,
                [" History (", hlen, ") / Queue (", qlen, ") "],
                colors.neutral,
                Some(widgets::Alignment::CenterHorizontal),
            );
        });

        if jb.is_empty() {
            widgets::print_ascii(
                queue_inner_area,
                buf,
                "No tracks in the queue",
                colors.neutral,
                Some(widgets::Alignment::Center),
            );
            return;
        }

        let scrolloff = (queue_inner_area.height / 2) as usize;
        self.list
            .set_margins(scrolloff, scrolloff)
            .set_padding(scrolloff);

        let hlen = jb.history();
        let current_qi = jb.current_queue_index();
        self.list.set_colors(colors.neutral, None).render(
            queue_inner_area,
            buf,
            jb.iter(),
            |line, buf, (id, qi), item| {
                let Some(track) = db.get(id) else {
                    return;
                };

                let mut style = if qi < hlen {
                    Style::new().fg(colors.neutral)
                } else if current_qi == Some(qi) {
                    Style::new().fg(colors.primary)
                } else {
                    Style::new()
                };

                if jb.is_faulty(id) {
                    style.add_modifier.insert(Modifier::CROSSED_OUT);
                }

                let symbol = match item {
                    ListItem::Selected => symbols::concat!(symbols::SELECTED, " "),
                    ListItem::Selection => symbols::concat!(symbols::SELECTION, " "),
                    ListItem::Normal => "",
                };

                widgets::print_texts_with_styles(
                    line,
                    buf,
                    [
                        (symbol, style.not_crossed_out()),
                        (track.title(), style),
                        (" ", style),
                        (track.artist(), style),
                        (" ", style),
                        (track.album(), style),
                    ],
                    Some(style.not_crossed_out()),
                    None,
                );
            },
        );
    }
}

fn render_cover_with_stars(
    area: Rect,
    buf: &mut Buffer,
    db: &Database,
    jb: &Jukebox,
    front_cover: &mut FrontCover,
    front_cover_status: FrontCoverStatus,
    colors: &Colors,
) {
    match jb
        .current_track_id()
        .and_then(|id| db.get(id).map(|track| track.rating()))
    {
        Some(rating) => {
            if area.width > 12 && area.height > 10 {
                let margin = Margin::new(1, 1);
                let cover_area = render_cover(
                    area.inner(margin),
                    buf,
                    front_cover,
                    front_cover_status,
                    colors,
                );
                let stars = symbols::stars_split(rating);
                widgets::print_texts_with_styles(
                    Rect {
                        y: cover_area.y + cover_area.height,
                        height: 1,
                        ..cover_area
                    },
                    buf,
                    [
                        (stars.0, Style::new().fg(colors.primary)),
                        (stars.1, Style::new().fg(colors.neutral)),
                    ],
                    None,
                    Some(widgets::Alignment::CenterHorizontal),
                );
            } else {
                render_cover(area, buf, front_cover, front_cover_status, colors);
            }
        }
        None => {
            widgets::print_ascii(
                area,
                buf,
                "No track currently playing",
                colors.neutral,
                Some(widgets::Alignment::Center),
            );
        }
    }
}

fn render_cover(
    area: Rect,
    buf: &mut Buffer,
    front_cover: &mut FrontCover,
    front_cover_status: FrontCoverStatus,
    colors: &Colors,
) -> Rect {
    let neutral_style = Style::new().fg(colors.neutral);

    const MAX_COVER_SIZE: u16 = 24;
    let mut image_area = {
        let s = area.width.min(area.height).min(MAX_COVER_SIZE);
        let w = area.width.min(s * 2);
        let h = s.min(w.div_ceil(2));
        let a = Rect {
            width: w,
            height: h,
            ..area
        };
        widgets::align(a, area, widgets::Alignment::Center)
    };

    match front_cover.as_mut() {
        Some(image) => {
            let resized_area = image.size_for(ratatui_image::Resize::default(), image_area);
            image_area = widgets::align(
                Rect {
                    width: resized_area.width,
                    height: resized_area.height,
                    ..area
                },
                area,
                widgets::Alignment::Center,
            );
            StatefulImage::default().render(image_area, buf, image);
        }
        None => match front_cover_status {
            FrontCoverStatus::None | FrontCoverStatus::Loading => {}
            FrontCoverStatus::Ready => {
                Block::bordered()
                    .style(neutral_style)
                    .render(image_area, buf);
                widgets::print_ascii(
                    image_area,
                    buf,
                    "NO IMAGE",
                    neutral_style,
                    Some(widgets::Alignment::Center),
                );
            }
        },
    }

    image_area
}
