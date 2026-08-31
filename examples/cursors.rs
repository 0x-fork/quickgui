use quickgui::{
    Application, Color, CursorStyle, Element, IntoElement, View, ViewContext, WindowOptions, div,
    text,
};

const CURSORS: [(&str, &str, CursorStyle); 21] = [
    ("Default", "cursor-default", CursorStyle::Arrow),
    ("Pointer", "cursor-pointer", CursorStyle::PointingHand),
    ("Text", "cursor-text", CursorStyle::IBeam),
    (
        "Vertical text",
        "cursor-vertical-text",
        CursorStyle::IBeamCursorForVerticalLayout,
    ),
    ("Crosshair", "cursor-crosshair", CursorStyle::Crosshair),
    ("Grab", "cursor-grab", CursorStyle::OpenHand),
    ("Grabbing", "cursor-grabbing", CursorStyle::ClosedHand),
    (
        "Not allowed",
        "cursor-not-allowed",
        CursorStyle::OperationNotAllowed,
    ),
    (
        "Context menu",
        "cursor-context-menu",
        CursorStyle::ContextualMenu,
    ),
    ("Alias", "cursor-alias", CursorStyle::DragLink),
    ("Copy", "cursor-copy", CursorStyle::DragCopy),
    ("Resize west", "cursor-w-resize", CursorStyle::ResizeLeft),
    ("Resize east", "cursor-e-resize", CursorStyle::ResizeRight),
    ("Resize north", "cursor-n-resize", CursorStyle::ResizeUp),
    ("Resize south", "cursor-s-resize", CursorStyle::ResizeDown),
    (
        "Resize horizontal",
        "cursor-ew-resize",
        CursorStyle::ResizeLeftRight,
    ),
    (
        "Resize vertical",
        "cursor-ns-resize",
        CursorStyle::ResizeUpDown,
    ),
    (
        "Resize NW/SE",
        "cursor-nwse-resize",
        CursorStyle::ResizeUpLeftDownRight,
    ),
    (
        "Resize NE/SW",
        "cursor-nesw-resize",
        CursorStyle::ResizeUpRightDownLeft,
    ),
    (
        "Resize column",
        "cursor-col-resize",
        CursorStyle::ResizeColumn,
    ),
    ("Resize row", "cursor-row-resize", CursorStyle::ResizeRow),
];

fn main() -> Result<(), quickgui::AppError> {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions::new("QuickGUI — native cursors")
                .size(900.0, 700.0)
                .minimum_size(660.0, 480.0)
                .background(Color::rgb8(14, 17, 23)),
            CursorGallery,
        );
    })
}

struct CursorGallery;

impl View for CursorGallery {
    fn render(&mut self, _cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        div()
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .flex_col()
            .gap_5()
            .bg(Color::rgb8(14, 17, 23))
            .text_color(Color::rgb8(237, 240, 246))
            .child(text("Native cursor styles").text_2xl().font_bold())
            .child(
                text("Hover each tile. Cursor changes are driven by pointer hit testing and do not schedule frames while the window is idle.")
                    .max_w(720.0)
                    .wrap()
                    .text_sm()
                    .text_color(Color::rgb8(158, 169, 188)),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .children(CURSORS.into_iter().map(|(label, css, cursor)| {
                        cursor_tile(label, css, cursor)
                    })),
            )
    }
}

fn cursor_tile(label: &'static str, css: &'static str, cursor: CursorStyle) -> Element {
    div()
        .cursor(cursor)
        .when(cursor == CursorStyle::OpenHand, |tile| {
            tile.clickable().active(|style| style.cursor_grabbing())
        })
        .min_h(92.0)
        .p_4()
        .flex_col()
        .justify_between()
        .rounded_xl()
        .border(1.0, Color::rgb8(57, 67, 84))
        .bg(Color::rgb8(22, 27, 36))
        .hover(|style| {
            style
                .bg(Color::rgb8(29, 37, 50))
                .border_color(Color::rgb8(96, 165, 250))
        })
        .child(text(label).font_semibold())
        .child(text(css).text_xs().text_color(Color::rgb8(125, 211, 252)))
}
