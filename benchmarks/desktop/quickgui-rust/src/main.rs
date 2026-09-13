use std::sync::Arc;

use quickgui::{
    App, AppInfo, Application, ClickListener, Color, Element, IntoElement, View, ViewContext,
    WindowAppearance, WindowOptions, button, div, text, text_area, text_input,
};
use serde::Deserialize;

const PAGE_SIZE: usize = 100;
const READY_TITLE: &str = "Issue tracker — ready";

fn ink() -> Color {
    Color::rgb8(32, 36, 44)
}

fn muted() -> Color {
    Color::rgb8(115, 124, 140)
}

fn line() -> Color {
    Color::rgb8(226, 229, 235)
}

fn accent() -> Color {
    Color::rgb8(40, 91, 212)
}

#[derive(Deserialize)]
struct IssueData {
    id: String,
    title: String,
    project: String,
    owner: String,
    priority: String,
    status: String,
    description: String,
    notes: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum IssueFilter {
    All,
    Open,
    Completed,
}

impl IssueFilter {
    const ALL: [Self; 3] = [Self::All, Self::Open, Self::Completed];

    const fn label(self) -> &'static str {
        match self {
            Self::All => "All issues",
            Self::Open => "Open",
            Self::Completed => "Completed",
        }
    }

    const fn id(self) -> &'static str {
        match self {
            Self::All => "filter-all",
            Self::Open => "filter-open",
            Self::Completed => "filter-completed",
        }
    }

    fn includes(self, issue: &IssueData) -> bool {
        match self {
            Self::All => true,
            Self::Open => issue.status != "Done",
            Self::Completed => issue.status == "Done",
        }
    }
}

struct IssueTracker {
    issues: Vec<IssueData>,
    query: String,
    filter: IssueFilter,
    page: usize,
    selected: usize,
}

impl IssueTracker {
    fn new() -> Self {
        let issues: Vec<IssueData> = serde_json::from_str(include_str!("../issues.generated.json"))
            .expect("the generated benchmark dataset is valid JSON");
        assert_eq!(issues.len(), 1_000, "expected 1,000 benchmark issues");
        Self {
            issues,
            query: String::new(),
            filter: IssueFilter::All,
            page: 0,
            selected: 0,
        }
    }

    fn matching_indices(&self) -> Vec<usize> {
        let needle = self.query.trim().to_lowercase();
        self.issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| self.filter.includes(issue))
            .filter(|(_, issue)| {
                needle.is_empty()
                    || issue.id.to_lowercase().contains(&needle)
                    || issue.title.to_lowercase().contains(&needle)
                    || issue.project.to_lowercase().contains(&needle)
                    || issue.owner.to_lowercase().contains(&needle)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn page_count(matches: usize) -> usize {
        matches.div_ceil(PAGE_SIZE).max(1)
    }

    fn caption(value: impl Into<Arc<str>>) -> Element {
        text(value)
            .text_size(12.0)
            .line_height(18.0)
            .text_color(muted())
    }

    fn property(label: &'static str, value: impl Into<Arc<str>>) -> Element {
        div()
            .flex_row()
            .items_center()
            .min_w(0.0)
            .justify_between()
            .child(Self::caption(label))
            .child(text(value).text_size(12.0).font_medium())
    }

    fn control(label: &'static str, disabled: bool, listener: ClickListener<Self>) -> Element {
        button()
            .on_click(listener)
            .disabled(disabled)
            .disabled_style(|style| style.opacity(0.4))
            .flex_row()
            .items_center()
            .min_w(0.0)
            .justify_center()
            .h(32.0)
            .px(12.0)
            .border(1.0, Color::rgb8(220, 224, 231))
            .rounded(6.0)
            .bg(Color::WHITE)
            .text_size(12.0)
            .child(label)
    }
}

impl View for IssueTracker {
    fn render(&mut self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let matching = self.matching_indices();
        let pages = Self::page_count(matching.len());
        self.page = self.page.min(pages - 1);
        let visible = matching
            .iter()
            .copied()
            .skip(self.page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect::<Vec<_>>();
        let completed = self
            .issues
            .iter()
            .filter(|issue| issue.status == "Done")
            .count();

        let search = cx.input_listener("search", |this, value, cx| {
            this.query.clear();
            this.query.push_str(value);
            this.page = 0;
            cx.invalidate();
        });
        let edit_notes = cx.input_listener("working-notes", |this, value, cx| {
            this.issues[this.selected].notes.clear();
            this.issues[this.selected].notes.push_str(value);
            cx.invalidate();
        });
        let previous = cx.listener("previous-page", |this, cx| {
            this.page = this.page.saturating_sub(1);
            cx.invalidate();
        });
        let next = cx.listener("next-page", |this, cx| {
            let pages = Self::page_count(this.matching_indices().len());
            this.page = (this.page + 1).min(pages - 1);
            cx.invalidate();
        });
        let complete = cx.listener("toggle-complete", |this, cx| {
            let status = &mut this.issues[this.selected].status;
            *status = if status == "Done" {
                "Open".to_owned()
            } else {
                "Done".to_owned()
            };
            cx.invalidate();
        });

        let mut filters = Vec::with_capacity(IssueFilter::ALL.len());
        for filter in IssueFilter::ALL {
            let selected = self.filter == filter;
            let choose = cx.listener(filter.id(), move |this, cx| {
                this.filter = filter;
                this.page = 0;
                cx.invalidate();
            });
            filters.push(
                button()
                    .on_click(choose)
                    .flex_row()
                    .items_center()
                    .min_w(0.0)
                    .w_full()
                    .h(42.0)
                    .flex_none()
                    .px(12.0)
                    .justify_start()
                    .rounded(7.0)
                    .font_normal()
                    .when(selected, |row| row.font_semibold())
                    .bg(if selected {
                        Color::rgb8(228, 235, 251)
                    } else {
                        Color::TRANSPARENT
                    })
                    .text_color(if selected { accent() } else { ink() })
                    .child(filter.label()),
            );
        }

        let sidebar = div()
            .flex_col()
            .min_h(0.0)
            .min_w(0.0)
            .w(176.0)
            .flex_none()
            .padding(24.0, 12.0, 24.0, 12.0)
            .gap(8.0)
            .bg(Color::rgb8(244, 245, 247))
            .border_right(1.0, line())
            .child(
                text("Orbit")
                    .text_size(22.0)
                    .font_bold()
                    .padding(0.0, 0.0, 0.0, 12.0)
                    .mb(6.0),
            )
            .child(
                Self::caption("Product workspace")
                    .padding(0.0, 0.0, 0.0, 12.0)
                    .mb(24.0),
            )
            .children(filters)
            .child(div().flex_1())
            .child(Self::caption("September cycle\n4 projects · 5 teammates").p(12.0));

        let header = div()
            .flex_row()
            .items_center()
            .min_w(0.0)
            .h(94.0)
            .flex_none()
            .px(24.0)
            .justify_between()
            .border_bottom(1.0, line())
            .child(
                div()
                    .flex_col()
                    .min_h(0.0)
                    .min_w(0.0)
                    .gap(6.0)
                    .child(text("Issue inbox").text_size(24.0).font_bold())
                    .child(Self::caption(format!(
                        "{} open · {completed} completed",
                        self.issues.len() - completed
                    ))),
            )
            .child(
                text_input(self.query.clone())
                    .on_input(search)
                    .placeholder("Search issues, projects, people")
                    .accessibility_label("Search issues")
                    .w(280.0)
                    .h(38.0)
                    .px(12.0)
                    .bg(Color::rgb8(248, 249, 251))
                    .border(1.0, Color::rgb8(220, 224, 231))
                    .rounded(7.0),
            );

        let mut rows = Vec::with_capacity(visible.len());
        for index in visible {
            let select = cx.listener(format!("select-issue-{index}"), move |this, cx| {
                this.selected = index;
                cx.invalidate();
            });
            let issue = &self.issues[index];
            rows.push(
                button()
                    .on_click(select)
                    .selected(self.selected == index)
                    .flex_col()
                    .items_stretch()
                    .min_w(0.0)
                    .w_full()
                    .h(68.0)
                    .flex_none()
                    .justify_center()
                    .gap(8.0)
                    .px(20.0)
                    .border_bottom(1.0, Color::rgb8(237, 240, 244))
                    .bg(if self.selected == index {
                        Color::rgb8(237, 243, 255)
                    } else {
                        Color::WHITE
                    })
                    .accessibility_label(issue.id.clone())
                    .child(
                        text(issue.title.clone())
                            .text_size(14.0)
                            .font_medium()
                            .no_wrap()
                            .text_ellipsis(),
                    )
                    .child(
                        text(format!(
                            "{}  ·  {}  ·  {}  ·  {}",
                            issue.id, issue.project, issue.status, issue.owner
                        ))
                        .text_size(11.0)
                        .text_color(muted())
                        .no_wrap(),
                    ),
            );
        }

        let list = if rows.is_empty() {
            div()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .child(Self::caption("No matching issues").p(20.0))
        } else {
            div().flex_col().flex_1().overflow_y_scroll().children(rows)
        };

        let inbox = div()
            .flex_col()
            .min_h(0.0)
            .min_w(0.0)
            .flex_1()
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .min_w(0.0)
                    .h(48.0)
                    .flex_none()
                    .px(20.0)
                    .justify_between()
                    .bg(Color::rgb8(250, 251, 252))
                    .border_bottom(1.0, line())
                    .child(Self::caption(format!("{} issues", matching.len())))
                    .child(Self::caption("Updated this week")),
            )
            .child(list)
            .child(
                div()
                    .flex_row()
                    .items_center()
                    .min_w(0.0)
                    .h(58.0)
                    .flex_none()
                    .px(20.0)
                    .justify_between()
                    .border_top(1.0, line())
                    .child(Self::caption(format!("Page {} of {pages}", self.page + 1)))
                    .child(
                        div()
                            .flex_row()
                            .items_center()
                            .min_w(0.0)
                            .gap(8.0)
                            .child(Self::control("Previous", self.page == 0, previous))
                            .child(Self::control("Next", self.page + 1 >= pages, next)),
                    ),
            );

        let current = &self.issues[self.selected];
        let details = div()
            .flex_col()
            .min_h(0.0)
            .min_w(0.0)
            .w(350.0)
            .flex_none()
            .overflow_y_scroll()
            .p(24.0)
            .border_left(1.0, line())
            .child(
                div()
                    .flex_col()
                    .min_h(0.0)
                    .min_w(0.0)
                    .flex_none()
                    .child(Self::caption(format!(
                        "{} / {}",
                        current.id, current.project
                    )))
                    .child(
                        text(current.title.clone())
                            .text_size(21.0)
                            .line_height(28.0)
                            .font_bold()
                            .mt(14.0)
                            .mb(22.0),
                    )
                    .child(
                        div()
                            .flex_col()
                            .min_h(0.0)
                            .min_w(0.0)
                            .gap(12.0)
                            .mb(24.0)
                            .child(Self::property("Status", current.status.clone()))
                            .child(Self::property("Assignee", current.owner.clone()))
                            .child(Self::property("Priority", current.priority.clone())),
                    )
                    .child(
                        text(current.description.clone())
                            .text_size(13.0)
                            .line_height(20.0)
                            .mb(22.0),
                    )
                    .child(
                        text("Working notes")
                            .text_size(12.0)
                            .font_semibold()
                            .mb(8.0),
                    )
                    .child(
                        text_area(current.notes.clone())
                            .on_input(edit_notes)
                            .accessibility_label("Working notes")
                            .h(100.0)
                            .flex_none()
                            .p(10.0)
                            .text_size(13.0)
                            .line_height(19.0)
                            .bg(Color::WHITE)
                            .text_color(ink())
                            .border(1.0, Color::rgb8(220, 224, 231))
                            .rounded(7.0),
                    )
                    .child(
                        button()
                            .on_click(complete)
                            .flex_row()
                            .items_center()
                            .min_w(0.0)
                            .h(36.0)
                            .flex_none()
                            .justify_center()
                            .rounded(7.0)
                            .mt(16.0)
                            .bg(accent())
                            .text_color(Color::WHITE)
                            .font_medium()
                            .child(if current.status == "Done" {
                                "Reopen issue"
                            } else {
                                "Mark complete"
                            }),
                    )
                    .child(
                        text("Changes are kept for this session.")
                            .text_size(11.0)
                            .text_color(muted())
                            .mt(10.0),
                    ),
            );

        div()
            .size_full()
            .flex_row()
            .items_stretch()
            .min_w(0.0)
            .bg(Color::WHITE)
            .text_color(ink())
            .text_size(14.0)
            .child(sidebar)
            .child(
                div()
                    .flex_col()
                    .min_h(0.0)
                    .min_w(0.0)
                    .flex_1()
                    .child(header)
                    .child(
                        div()
                            .flex_row()
                            .items_stretch()
                            .min_h(0.0)
                            .min_w(0.0)
                            .flex_1()
                            .child(inbox)
                            .child(details),
                    ),
            )
    }
}

fn open_window(cx: &mut App) {
    cx.open_window(
        WindowOptions::new(READY_TITLE)
            .size(1_100.0, 720.0)
            .window_appearance(WindowAppearance::Light),
        IssueTracker::new(),
    );
}

fn main() -> Result<(), quickgui::AppError> {
    Application::new()
        .app_info(
            AppInfo::new(
                "Benchmark QuickGUI Rust",
                "0.1.0",
                "dev.quickgui.benchmark.rust",
            )
            .expect("valid benchmark application identity"),
        )
        .on_reopen(|has_visible_windows, cx| {
            if !has_visible_windows {
                open_window(cx);
            }
        })
        .run(open_window)
}
