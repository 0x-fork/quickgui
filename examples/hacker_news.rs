use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use quickgui::{
    Application, Color, Element, ElementId, Event, EventContext, Key, ListState, TitleBarStyle,
    View, ViewContext, VirtualList, button, div, text,
};
use serde::Deserialize;

const API_ROOT: &str = "https://hacker-news.firebaseio.com/v0";
const STORY_LIMIT: usize = 32;
const COMMENT_LIMIT: usize = 96;
const COMMENT_FETCH_LIMIT: usize = 128;
const COMMENT_DEPTH_LIMIT: u16 = 10;
const API_CONCURRENCY: usize = 8;

const APP_HEADER_HEIGHT: f32 = 64.0;
const PANEL_HEADER_HEIGHT: f32 = 62.0;
const STORY_ROW_HEIGHT: f32 = 94.0;
const ESTIMATED_COMMENT_ROW_HEIGHT: f32 = 104.0;
const STORY_PANEL_FRACTION: f32 = 0.43;
const REFRESH_ID: u64 = 0x2100_0000_0000_0001;
const COMMENTS_SCROLL_ID: u64 = 0x2100_0000_0000_0002;
const STORIES_SCROLL_ID: u64 = 0x2100_0000_0000_0003;
const STORY_ID_BASE: u64 = 0x2200_0000_0000_0000;

fn main() -> Result<(), quickgui::AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    Application::new().run(|cx| {
        cx.open_window(
            quickgui::WindowOptions::new("QuickGUI — Hacker News")
                .size(1_280.0, 800.0)
                .title_bar_style(TitleBarStyle::HiddenInset)
                .traffic_light_position(16.0, 13.0),
            HackerNews::new(),
        );
    })
}

struct Story {
    id: u64,
    title: Arc<str>,
    domain: Arc<str>,
    meta: Arc<str>,
    comments_label: Arc<str>,
    kids: Arc<[u64]>,
}

struct CommentRow {
    author: Arc<str>,
    meta: Arc<str>,
    body: Arc<str>,
    depth: u16,
}

enum FetchResult<T> {
    Ready(T),
    Failed(Arc<str>),
}

enum FeedState {
    NotStarted,
    Loading,
    Ready,
    Failed(Arc<str>),
}

enum CommentState {
    Idle,
    Queued {
        story_id: u64,
        kids: Arc<[u64]>,
        generation: u64,
    },
    Loading {
        story_id: u64,
        generation: u64,
    },
    Ready {
        story_id: u64,
    },
    Failed {
        story_id: u64,
        message: Arc<str>,
    },
}

struct HackerNews {
    feed_state: FeedState,
    comment_state: CommentState,
    stories: Vec<Story>,
    comments: Vec<CommentRow>,
    story_list: VirtualList,
    comment_list: ListState,
    selected_story: Option<usize>,
    split_x: f32,
    comment_generation: Arc<AtomicU64>,
}

impl HackerNews {
    fn new() -> Self {
        Self {
            feed_state: FeedState::NotStarted,
            comment_state: CommentState::Idle,
            stories: Vec::new(),
            comments: Vec::new(),
            story_list: VirtualList::new(0, STORY_ROW_HEIGHT).with_overscan(2),
            comment_list: ListState::new(0, ESTIMATED_COMMENT_ROW_HEIGHT).with_overscan(3),
            selected_story: None,
            split_x: 0.0,
            comment_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    fn refresh(&mut self) {
        self.comment_generation.fetch_add(1, Ordering::Relaxed);
        self.feed_state = FeedState::NotStarted;
        self.comment_state = CommentState::Idle;
        self.stories.clear();
        self.comments.clear();
        self.story_list.set_len(0);
        self.story_list.scroll_to(0.0);
        self.comment_list.reset(0);
        self.selected_story = None;
    }

    fn select_story(&mut self, index: usize) {
        let Some(story) = self.stories.get(index) else {
            return;
        };
        self.selected_story = Some(index);
        self.comments.clear();
        self.comment_list.reset(0);
        let generation = self
            .comment_generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        self.comment_state = CommentState::Queued {
            story_id: story.id,
            kids: story.kids.clone(),
            generation,
        };
    }

    fn select_and_reveal_story(&mut self, index: usize) {
        if self.stories.is_empty() {
            return;
        }
        let index = index.min(self.stories.len() - 1);
        let top = index as f32 * STORY_ROW_HEIGHT;
        let bottom = top + STORY_ROW_HEIGHT;
        if top < self.story_list.scroll_offset() {
            self.story_list.scroll_to(top);
        } else if bottom > self.story_list.scroll_offset() + self.story_list.viewport_height() {
            self.story_list
                .scroll_to(bottom - self.story_list.viewport_height());
        }
        self.select_story(index);
    }

    fn start_pending_tasks(&mut self, cx: &ViewContext<'_, Self>) {
        if matches!(self.feed_state, FeedState::NotStarted) {
            self.feed_state = FeedState::Loading;
            if let Err(error) = cx.spawn_background(fetch_front_page, |this, result, event_cx| {
                match result {
                    Ok(FetchResult::Ready(stories)) => {
                        this.stories = stories;
                        this.story_list.set_len(this.stories.len());
                        this.feed_state = FeedState::Ready;
                        if !this.stories.is_empty() {
                            this.select_story(0);
                        }
                    }
                    Ok(FetchResult::Failed(message)) => {
                        this.feed_state = FeedState::Failed(message);
                    }
                    Err(error) => {
                        this.feed_state = FeedState::Failed(Arc::from(error.to_string()));
                    }
                }
                event_cx.invalidate();
            }) {
                self.feed_state = FeedState::Failed(Arc::from(error.to_string()));
            }
        }

        let queued = match &self.comment_state {
            CommentState::Queued {
                story_id,
                kids,
                generation,
            } => Some((*story_id, kids.clone(), *generation)),
            _ => None,
        };
        if let Some((story_id, kids, generation)) = queued {
            self.comment_state = CommentState::Loading {
                story_id,
                generation,
            };
            let current_generation = self.comment_generation.clone();
            if let Err(error) = cx.spawn_background(
                move || fetch_comments(kids, current_generation, generation),
                move |this, result, event_cx| {
                    // Ignore a stale completion after the user has selected another story.
                    if !matches!(
                        this.comment_state,
                        CommentState::Loading {
                            story_id: current,
                            generation: current_generation,
                        } if current == story_id && current_generation == generation
                    ) {
                        return;
                    }
                    match result {
                        Ok(FetchResult::Ready(comments)) => {
                            this.comments = comments;
                            this.comment_list.set_item_count(this.comments.len());
                            this.comment_state = CommentState::Ready { story_id };
                        }
                        Ok(FetchResult::Failed(message)) => {
                            this.comment_state = CommentState::Failed { story_id, message };
                        }
                        Err(error) => {
                            this.comment_state = CommentState::Failed {
                                story_id,
                                message: Arc::from(error.to_string()),
                            };
                        }
                    }
                    event_cx.invalidate();
                },
            ) {
                self.comment_state = CommentState::Failed {
                    story_id,
                    message: Arc::from(error.to_string()),
                };
            }
        }
    }

    fn story_row(&self, index: usize) -> Element {
        let story = &self.stories[index];
        let selected = self.selected_story == Some(index);
        div()
            .id(ElementId::new(STORY_ID_BASE + index as u64))
            .clickable()
            .absolute()
            .top(index as f32 * STORY_ROW_HEIGHT - self.story_list.scroll_offset())
            .left(0.0)
            .w_full()
            .h(STORY_ROW_HEIGHT)
            .flex_row()
            .items_start()
            .padding(13.0, 22.0, 12.0, 14.0)
            .gap_3()
            .bg(if selected {
                Color::rgb8(39, 45, 55)
            } else {
                Color::rgb8(24, 26, 31)
            })
            .border(1.0, Color::rgb8(42, 45, 52))
            .hover(|style| style.bg(Color::rgb8(34, 37, 44)))
            .active(|style| style.bg(Color::rgb8(48, 51, 59)))
            .child(
                text(Arc::from(format!("{}", index + 1)))
                    .w(24.0)
                    .flex_none()
                    .text_sm()
                    .no_wrap()
                    .text_color(Color::rgb8(126, 131, 143)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(0.0)
                    .flex_col()
                    .gap_1()
                    .child(
                        text(story.title.clone())
                            .w_full()
                            .h(42.0)
                            .overflow_hidden()
                            .wrap()
                            .text_base()
                            .font_semibold()
                            .text_color(Color::rgb8(239, 240, 243)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                text(story.meta.clone())
                                    .flex_1()
                                    .min_w(0.0)
                                    .text_xs()
                                    .no_wrap()
                                    .overflow_hidden()
                                    .text_color(Color::rgb8(151, 156, 168)),
                            )
                            .child(
                                text(story.comments_label.clone())
                                    .flex_none()
                                    .text_xs()
                                    .no_wrap()
                                    .text_color(Color::rgb8(255, 102, 0)),
                            ),
                    )
                    .when(!story.domain.is_empty(), |element| {
                        element.child(
                            text(story.domain.clone())
                                .text_xs()
                                .no_wrap()
                                .text_color(Color::rgb8(115, 121, 134)),
                        )
                    }),
            )
    }

    fn comment_row(&self, index: usize) -> Element {
        let comment = &self.comments[index];
        let indent = f32::from(comment.depth.min(8)) * 14.0;
        div()
            .flex_none()
            .w_full()
            .padding(13.0, 24.0, 12.0, 18.0 + indent)
            .flex_col()
            .gap_2()
            .border(1.0, Color::rgb8(42, 45, 52))
            .bg(if index.is_multiple_of(2) {
                Color::rgb8(21, 23, 27)
            } else {
                Color::rgb8(24, 26, 31)
            })
            .hover(|style| style.bg(Color::rgb8(31, 34, 40)))
            .child(
                div()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        text(comment.author.clone())
                            .text_sm()
                            .font_semibold()
                            .no_wrap()
                            .text_color(Color::rgb8(244, 149, 84)),
                    )
                    .child(
                        text(comment.meta.clone())
                            .text_xs()
                            .no_wrap()
                            .text_color(Color::rgb8(126, 131, 143)),
                    ),
            )
            .child(
                text(comment.body.clone())
                    .w_full()
                    .wrap()
                    .text_sm()
                    .line_height(19.0)
                    .text_color(Color::rgb8(211, 214, 221)),
            )
    }

    fn feed_status(&self) -> Arc<str> {
        match &self.feed_state {
            FeedState::NotStarted | FeedState::Loading => Arc::from("Loading top stories…"),
            FeedState::Ready => Arc::from(format!("{} top stories", self.stories.len())),
            FeedState::Failed(message) => Arc::from(format!("Could not load: {message}")),
        }
    }

    fn comments_status(&self) -> Arc<str> {
        match &self.comment_state {
            CommentState::Idle => Arc::from("Select a story"),
            CommentState::Queued { .. } | CommentState::Loading { .. } => {
                Arc::from("Loading discussion…")
            }
            CommentState::Ready { story_id } => Arc::from(format!(
                "{} comments loaded · story {story_id}",
                self.comments.len()
            )),
            CommentState::Failed { story_id, message } => {
                Arc::from(format!("Story {story_id}: {message}"))
            }
        }
    }
}

impl View for HackerNews {
    fn event(&mut self, event: &Event, cx: &mut EventContext) {
        match event {
            Event::Click(id) if id.as_u64() == REFRESH_ID => {
                self.refresh();
                cx.invalidate();
            }
            Event::Click(id)
                if (STORY_ID_BASE..STORY_ID_BASE + self.stories.len() as u64)
                    .contains(&id.as_u64()) =>
            {
                self.select_story((id.as_u64() - STORY_ID_BASE) as usize);
                cx.invalidate();
            }
            Event::KeyDown { key, .. } => match key {
                Key::Escape => cx.exit(),
                Key::Character(value) if value.eq_ignore_ascii_case("r") => {
                    self.refresh();
                    cx.invalidate();
                }
                Key::ArrowUp => {
                    let next = self.selected_story.unwrap_or(0).saturating_sub(1);
                    self.select_and_reveal_story(next);
                    cx.invalidate();
                }
                Key::ArrowDown => {
                    let next = self
                        .selected_story
                        .map_or(0, |index| index.saturating_add(1));
                    self.select_and_reveal_story(next);
                    cx.invalidate();
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl quickgui::IntoElement {
        self.start_pending_tasks(cx);

        let size = cx.size();
        self.split_x = size.width * STORY_PANEL_FRACTION;
        let viewport_height = (size.height - APP_HEADER_HEIGHT - PANEL_HEADER_HEIGHT).max(0.0);
        self.story_list.set_viewport_height(viewport_height);
        let comment_viewport_width = (size.width - self.split_x - 1.0).max(0.0);
        self.comment_list
            .set_viewport_size(comment_viewport_width, viewport_height);

        let story_rows = self
            .story_list
            .visible_rows()
            .range
            .map(|index| self.story_row(index))
            .collect::<Vec<_>>();
        let visible_comments = self.comment_list.visible_rows();
        let comment_rows = self
            .comment_list
            .render_rows(visible_comments.range, |index| self.comment_row(index));

        let selected_title = self
            .selected_story
            .and_then(|index| self.stories.get(index))
            .map_or_else(|| Arc::from("Discussion"), |story| story.title.clone());

        div()
            .size_full()
            .flex_col()
            .bg(Color::rgb8(18, 19, 22))
            .text_color(Color::rgb8(221, 223, 228))
            .child(
                div()
                    .h(APP_HEADER_HEIGHT)
                    .w_full()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .padding(0.0, 16.0, 0.0, 86.0)
                    .bg(Color::rgb8(255, 102, 0))
                    .app_region_drag()
                    .child(
                        div()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(34.0, 34.0)
                                    .flex_row()
                                    .items_center()
                                    .justify_center()
                                    .border(2.0, Color::WHITE)
                                    .child(text("Y").text_xl().text_color(Color::WHITE)),
                            )
                            .child(
                                div()
                                    .flex_col()
                                    .child(
                                        text("Hacker News")
                                            .text_xl()
                                            .font_bold()
                                            .text_color(Color::WHITE),
                                    )
                                    .child(
                                        text("QuickGUI live API example")
                                            .text_xs()
                                            .text_color(Color::rgba8(255, 255, 255, 205)),
                                    ),
                            ),
                    )
                    .child(
                        button()
                            .id(ElementId::new(REFRESH_ID))
                            .app_region_no_drag()
                            .clickable()
                            .h(36.0)
                            .flex_row()
                            .items_center()
                            .px_4()
                            .rounded_lg()
                            .border(1.0, Color::rgba8(255, 255, 255, 132))
                            .bg(Color::rgba8(255, 255, 255, 28))
                            .hover(|style| style.bg(Color::rgba8(255, 255, 255, 55)))
                            .active(|style| style.bg(Color::rgba8(0, 0, 0, 30)))
                            .child(text("Refresh  R").font_semibold().text_color(Color::WHITE)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .w_full()
                    .flex_row()
                    .child(
                        div()
                            .w(self.split_x)
                            .flex_none()
                            .h_full()
                            .flex_col()
                            .child(panel_header("Top stories", self.feed_status()))
                            .child(
                                div()
                                    .id(ElementId::new(STORIES_SCROLL_ID))
                                    .relative()
                                    .flex_1()
                                    .min_h(0.0)
                                    .w_full()
                                    .overflow_hidden()
                                    .virtual_scroll(&self.story_list)
                                    .children(story_rows),
                            ),
                    )
                    .child(
                        div()
                            .w(1.0)
                            .h_full()
                            .flex_none()
                            .bg(Color::rgb8(58, 61, 70)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(0.0)
                            .h_full()
                            .flex_col()
                            .child(panel_header(selected_title, self.comments_status()))
                            .child(
                                div()
                                    .id(ElementId::new(COMMENTS_SCROLL_ID))
                                    .relative()
                                    .flex_1()
                                    .min_h(0.0)
                                    .w_full()
                                    .overflow_hidden()
                                    .variable_virtual_scroll(&self.comment_list)
                                    .child(comment_rows)
                                    .when(self.comments.is_empty(), |element| {
                                        element.child(
                                            div()
                                                .absolute()
                                                .top(0.0)
                                                .left(0.0)
                                                .size_full()
                                                .flex_row()
                                                .items_center()
                                                .justify_center()
                                                .child(
                                                    text(self.comments_status())
                                                        .text_sm()
                                                        .text_color(Color::rgb8(133, 138, 151)),
                                                ),
                                        )
                                    }),
                            ),
                    ),
            )
    }
}

fn panel_header(title: impl Into<Arc<str>>, status: impl Into<Arc<str>>) -> Element {
    div()
        .h(PANEL_HEADER_HEIGHT)
        .w_full()
        .flex_none()
        .flex_col()
        .justify_center()
        .px_4()
        .gap_1()
        .border(1.0, Color::rgb8(47, 50, 58))
        .bg(Color::rgb8(29, 31, 36))
        .child(
            text(title.into())
                .w_full()
                .no_wrap()
                .overflow_hidden()
                .font_semibold()
                .text_color(Color::rgb8(239, 240, 243)),
        )
        .child(
            text(status.into())
                .w_full()
                .no_wrap()
                .overflow_hidden()
                .text_xs()
                .text_color(Color::rgb8(139, 144, 156)),
        )
}

#[derive(Deserialize)]
struct HnItem {
    id: u64,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    time: Option<u64>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    dead: bool,
    #[serde(default)]
    kids: Vec<u64>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    score: Option<u64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    descendants: Option<u64>,
}

fn fetch_front_page() -> FetchResult<Vec<Story>> {
    match try_fetch_front_page() {
        Ok(stories) => FetchResult::Ready(stories),
        Err(error) => FetchResult::Failed(Arc::from(error)),
    }
}

fn try_fetch_front_page() -> Result<Vec<Story>, String> {
    let agent = api_agent();
    let ids: Vec<u64> = fetch_json(&agent, &format!("{API_ROOT}/topstories.json"))?;
    let mut stories = Vec::with_capacity(STORY_LIMIT);
    let ids = ids.into_iter().take(STORY_LIMIT + 8).collect::<Vec<_>>();
    for (id, result) in ids.iter().copied().zip(fetch_items(&agent, &ids)) {
        let item = match result {
            Ok(item) => item,
            Err(error) => {
                tracing::warn!(item = id, %error, "Hacker News story could not be loaded");
                continue;
            }
        };
        let Some(title) = item.title.filter(|title| !title.trim().is_empty()) else {
            continue;
        };
        let author = item.by.as_deref().unwrap_or("unknown");
        let score = item.score.unwrap_or(0);
        let age = relative_age(item.time.unwrap_or(0));
        let descendants = item.descendants.unwrap_or(item.kids.len() as u64);
        stories.push(Story {
            id: item.id,
            title: html_to_text(&title),
            domain: item.url.as_deref().map(domain).unwrap_or_default(),
            meta: Arc::from(format!("{score} points · {author} · {age}")),
            comments_label: Arc::from(format!("{descendants} comments")),
            kids: item.kids.into(),
        });
        if stories.len() == STORY_LIMIT {
            break;
        }
    }
    if stories.is_empty() {
        Err("the API returned no readable stories".to_owned())
    } else {
        Ok(stories)
    }
}

fn fetch_comments(
    kids: Arc<[u64]>,
    current_generation: Arc<AtomicU64>,
    generation: u64,
) -> FetchResult<Vec<CommentRow>> {
    match try_fetch_comments(&kids, &current_generation, generation) {
        Ok(comments) => FetchResult::Ready(comments),
        Err(error) => FetchResult::Failed(Arc::from(error)),
    }
}

struct PendingComment {
    id: u64,
    depth: u16,
    path: Vec<u16>,
}

fn try_fetch_comments(
    kids: &[u64],
    current_generation: &AtomicU64,
    generation: u64,
) -> Result<Vec<CommentRow>, String> {
    if kids.is_empty() {
        return Ok(Vec::new());
    }
    let agent = api_agent();
    let mut frontier = kids
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| PendingComment {
            id,
            depth: 0,
            path: vec![index.min(u16::MAX as usize) as u16],
        })
        .collect::<Vec<_>>();
    let mut comments = Vec::with_capacity(COMMENT_LIMIT);
    let mut fetched = 0;
    let mut failures = 0;
    while !frontier.is_empty() && fetched < COMMENT_FETCH_LIMIT {
        if current_generation.load(Ordering::Relaxed) != generation {
            return Ok(Vec::new());
        }
        frontier.truncate(COMMENT_FETCH_LIMIT - fetched);
        let ids = frontier
            .iter()
            .map(|pending| pending.id)
            .collect::<Vec<_>>();
        let results = fetch_items(&agent, &ids);
        fetched += frontier.len();
        let mut next_frontier = Vec::new();
        for (pending, result) in frontier.drain(..).zip(results) {
            let item = match result {
                Ok(item) => item,
                Err(error) => {
                    failures += 1;
                    tracing::warn!(item = pending.id, %error, "Hacker News comment could not be loaded");
                    continue;
                }
            };
            if pending.depth < COMMENT_DEPTH_LIMIT {
                next_frontier.extend(item.kids.iter().copied().enumerate().map(
                    |(index, child)| {
                        let mut path = pending.path.clone();
                        path.push(index.min(u16::MAX as usize) as u16);
                        PendingComment {
                            id: child,
                            depth: pending.depth + 1,
                            path,
                        }
                    },
                ));
            }
            if item.kind != "comment" || item.deleted || item.dead {
                continue;
            }
            let Some(raw_body) = item.text.as_deref() else {
                continue;
            };
            let body = html_to_text(raw_body);
            if body.trim().is_empty() {
                continue;
            }
            comments.push((
                pending.path,
                CommentRow {
                    author: Arc::from(item.by.as_deref().unwrap_or("unknown")),
                    meta: Arc::from(relative_age(item.time.unwrap_or(0))),
                    body,
                    depth: pending.depth,
                },
            ));
        }
        frontier = next_frontier;
        if comments.len() >= COMMENT_LIMIT {
            break;
        }
    }
    if comments.is_empty() && failures > 0 {
        Err("the discussion could not be loaded".to_owned())
    } else {
        comments.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        Ok(comments
            .into_iter()
            .take(COMMENT_LIMIT)
            .map(|(_, comment)| comment)
            .collect())
    }
}

fn api_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}

fn fetch_json<T: serde::de::DeserializeOwned>(agent: &ureq::Agent, url: &str) -> Result<T, String> {
    agent
        .get(url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_json()
        .map_err(|error| error.to_string())
}

fn fetch_items(agent: &ureq::Agent, ids: &[u64]) -> Vec<Result<HnItem, String>> {
    if ids.is_empty() {
        return Vec::new();
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(ids.len())
            .collect::<Vec<Option<Result<HnItem, String>>>>(),
    );
    let worker_count = ids.len().min(API_CONCURRENCY);
    thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(id) = ids.get(index).copied() else {
                        return;
                    };
                    let result = fetch_json(agent, &format!("{API_ROOT}/item/{id}.json"));
                    results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())[index] = Some(result);
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .into_iter()
        .map(|result| result.expect("every scheduled API item produced a result"))
        .collect()
}

fn domain(url: &str) -> Arc<str> {
    let host = url
        .split_once("://")
        .map_or(url, |(_, remainder)| remainder)
        .split('/')
        .next()
        .unwrap_or("")
        .trim_start_matches("www.");
    Arc::from(host)
}

fn relative_age(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(timestamp, |duration| duration.as_secs());
    let elapsed = now.saturating_sub(timestamp);
    if elapsed < 60 {
        "just now".to_owned()
    } else if elapsed < 3_600 {
        plural(elapsed / 60, "minute")
    } else if elapsed < 86_400 {
        plural(elapsed / 3_600, "hour")
    } else {
        plural(elapsed / 86_400, "day")
    }
}

fn plural(value: u64, unit: &str) -> String {
    format!("{value} {unit}{} ago", if value == 1 { "" } else { "s" })
}

fn html_to_text(value: &str) -> Arc<str> {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '<' => {
                let mut tag = String::new();
                for character in chars.by_ref() {
                    if character == '>' {
                        break;
                    }
                    if tag.len() < 16 {
                        tag.push(character.to_ascii_lowercase());
                    }
                }
                let name = tag
                    .trim()
                    .trim_start_matches('/')
                    .split_ascii_whitespace()
                    .next()
                    .unwrap_or("");
                if matches!(name, "p" | "br" | "pre" | "li") && !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            '&' => {
                let mut entity = String::new();
                let mut terminated = false;
                while entity.len() < 12 {
                    let Some(next) = chars.next() else {
                        break;
                    };
                    if next == ';' {
                        terminated = true;
                        break;
                    }
                    if next.is_whitespace() || next == '<' {
                        entity.push(next);
                        break;
                    }
                    entity.push(next);
                }
                if terminated {
                    if let Some(decoded) = decode_entity(&entity) {
                        output.push(decoded);
                    } else {
                        output.push('&');
                        output.push_str(&entity);
                        output.push(';');
                    }
                } else {
                    output.push('&');
                    output.push_str(&entity);
                }
            }
            '\r' => {}
            _ => output.push(character),
        }
    }

    let mut normalized = String::with_capacity(output.len());
    let mut previous_blank = false;
    for line in output.lines() {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if !normalized.is_empty() && !previous_blank {
                normalized.push('\n');
                previous_blank = true;
            }
            continue;
        }
        if !normalized.is_empty() && !normalized.ends_with('\n') {
            normalized.push('\n');
        }
        normalized.push_str(&line);
        previous_blank = false;
    }
    Arc::from(normalized.trim())
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hacker_news_html_is_readable_and_decoded() {
        assert_eq!(
            html_to_text("Hello &amp; goodbye<p><i>second</i> line&#33;").as_ref(),
            "Hello & goodbye\nsecond line!"
        );
    }

    #[test]
    #[ignore = "uses the live Hacker News API"]
    fn live_api_loads_a_story_and_its_discussion() {
        let stories = try_fetch_front_page().expect("top stories should load");
        let story = stories
            .iter()
            .find(|story| !story.kids.is_empty())
            .expect("the front page should contain a discussion");
        let generation = AtomicU64::new(1);
        let comments = try_fetch_comments(&story.kids, &generation, 1)
            .expect("the selected discussion should load");
        assert!(!comments.is_empty());
    }
}
