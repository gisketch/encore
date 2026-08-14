//! Where the post-save preview sits on screen, as pure geometry.
//!
//! Kept free of Tauri types so the placement rule — inside the monitor
//! work area, with a margin, clear of the floating bar — is testable
//! without a live `AppHandle` or a real monitor. `preview::window` reads
//! the numbers off the monitor and hands them here.

/// A rectangle in physical screen pixels, matching what
/// `Monitor::work_area` and `WebviewWindow::outer_size` report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Places a `size`-sized window in the work area's bottom-right corner,
/// `margin` away from both edges.
///
/// `reserved_bottom` is the vertical band along the bottom edge the
/// preview must stay clear of — the floating bar's height plus its own
/// margin. The bar is centered horizontally (see
/// `desktop::position_floating_window`), so on a wide monitor a
/// bottom-right preview would already miss it, but on a narrow one the
/// 760pt bar reaches far enough right to collide. Lifting the preview
/// above the bar's band makes the guarantee independent of monitor width
/// instead of true only by luck.
///
/// Both offsets are clamped at the work area's own origin, so a window
/// larger than the space available lands at the top-left of the work area
/// rather than off-screen.
pub(crate) fn bottom_right(
    work_area: Rect,
    size: (i32, i32),
    margin: i32,
    reserved_bottom: i32,
) -> (i32, i32) {
    let x = work_area.x + (work_area.width - size.0 - margin).max(0);
    let y = work_area.y + (work_area.height - size.1 - margin - reserved_bottom).max(0);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARGIN: i32 = 16;
    const PREVIEW: (i32, i32) = (320, 250);
    const BAR: (i32, i32) = (760, 84);

    impl Rect {
        fn overlaps(&self, other: &Rect) -> bool {
            self.x < other.x + other.width
                && other.x < self.x + self.width
                && self.y < other.y + other.height
                && other.y < self.y + self.height
        }

        fn contains(&self, other: &Rect) -> bool {
            other.x >= self.x
                && other.y >= self.y
                && other.x + other.width <= self.x + self.width
                && other.y + other.height <= self.y + self.height
        }
    }

    /// The floating bar's rectangle, mirroring
    /// `desktop::position_floating_window`: horizontally centered in the
    /// work area, one margin above its bottom edge.
    fn bar_rect(work_area: Rect) -> Rect {
        Rect {
            x: work_area.x + (work_area.width - BAR.0).max(0) / 2,
            y: work_area.y + work_area.height - BAR.1 - MARGIN,
            width: BAR.0,
            height: BAR.1,
        }
    }

    fn preview_rect(work_area: Rect) -> Rect {
        let (x, y) = bottom_right(work_area, PREVIEW, MARGIN, BAR.1 + MARGIN);
        Rect {
            x,
            y,
            width: PREVIEW.0,
            height: PREVIEW.1,
        }
    }

    /// Work areas worth checking: a laptop display, a wide external one,
    /// a monitor whose work area is offset from the origin (a second
    /// display), and one narrow enough that the centered bar reaches into
    /// the right-hand corner.
    fn work_areas() -> [Rect; 4] {
        [
            Rect {
                x: 0,
                y: 25,
                width: 1440,
                height: 875,
            },
            Rect {
                x: 0,
                y: 25,
                width: 3440,
                height: 1415,
            },
            Rect {
                x: 1440,
                y: 0,
                width: 1920,
                height: 1080,
            },
            Rect {
                x: 0,
                y: 25,
                width: 1024,
                height: 743,
            },
        ]
    }

    #[test]
    fn the_preview_sits_inside_the_work_area_with_its_margin() {
        for work_area in work_areas() {
            let preview = preview_rect(work_area);
            assert!(
                work_area.contains(&preview),
                "{preview:?} escaped {work_area:?}"
            );
            assert_eq!(
                preview.x + preview.width,
                work_area.x + work_area.width - MARGIN,
                "right margin wrong for {work_area:?}"
            );
            assert!(
                preview.y > work_area.y,
                "preview hugged the top of {work_area:?}"
            );
        }
    }

    #[test]
    fn the_preview_never_overlaps_the_floating_bar() {
        for work_area in work_areas() {
            let preview = preview_rect(work_area);
            let bar = bar_rect(work_area);
            assert!(
                !preview.overlaps(&bar),
                "preview {preview:?} covered bar {bar:?} in {work_area:?}"
            );
        }
    }

    #[test]
    fn a_narrow_work_area_would_collide_without_the_reserved_band() {
        // Guards the reason `reserved_bottom` exists: with it dropped,
        // the bottom-right corner is inside the centered bar on a 1024pt
        // work area, so this is not a rule that horizontal placement
        // alone would satisfy.
        let work_area = work_areas()[3];
        let (x, y) = bottom_right(work_area, PREVIEW, MARGIN, 0);
        let unreserved = Rect {
            x,
            y,
            width: PREVIEW.0,
            height: PREVIEW.1,
        };
        assert!(unreserved.overlaps(&bar_rect(work_area)));
    }

    #[test]
    fn a_window_larger_than_its_work_area_stays_at_the_work_area_origin() {
        let work_area = Rect {
            x: 100,
            y: 50,
            width: 200,
            height: 200,
        };
        assert_eq!(
            bottom_right(work_area, PREVIEW, MARGIN, 0),
            (work_area.x, work_area.y)
        );
    }
}
