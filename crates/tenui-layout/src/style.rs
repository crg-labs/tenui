pub use taffy::prelude::*;

/// Ergonomic style extensions for Taffy's `Style` struct.
pub trait StyleExt {
    fn flex_row(self) -> Self;
    fn flex_col(self) -> Self;
    fn grow(self, val: f32) -> Self;
    fn shrink(self, val: f32) -> Self;
    fn width_cells(self, cells: f32) -> Self;
    fn height_cells(self, cells: f32) -> Self;
    fn width_percent(self, pct: f32) -> Self;
    fn height_percent(self, pct: f32) -> Self;
    fn padding_all(self, cells: f32) -> Self;
    fn margin_all(self, cells: f32) -> Self;
    fn center(self) -> Self;
    fn safe_center(self) -> Self;
    fn display_grid(self) -> Self;
    fn grid_columns(self, count: u16) -> Self;
    fn grid_rows(self, count: u16) -> Self;
    fn grid_template_columns(self, tracks: Vec<TrackSizingFunction>) -> Self;
    fn grid_template_rows(self, tracks: Vec<TrackSizingFunction>) -> Self;
    fn grid_col_span(self, span_count: u16) -> Self;
    fn grid_row_span(self, span_count: u16) -> Self;
    fn grid_cell(self, row: i16, col: i16) -> Self;
    fn gap_cells(self, w: f32, h: f32) -> Self;
    fn min_content_width(self) -> Self;
    fn min_content_height(self) -> Self;
    fn min_content(self) -> Self;
    fn max_content_width(self) -> Self;
    fn max_content_height(self) -> Self;
    fn max_content(self) -> Self;
    fn auto_width(self) -> Self;
    fn auto_height(self) -> Self;
    fn auto_size(self) -> Self;
}

impl StyleExt for Style {
    fn flex_row(mut self) -> Self {
        self.flex_direction = FlexDirection::Row;
        self
    }

    fn flex_col(mut self) -> Self {
        self.flex_direction = FlexDirection::Column;
        self
    }

    fn grow(mut self, val: f32) -> Self {
        self.flex_grow = val;
        self
    }

    fn shrink(mut self, val: f32) -> Self {
        self.flex_shrink = val;
        self
    }

    fn width_cells(mut self, cells: f32) -> Self {
        self.size.width = Dimension::length(cells);
        self
    }

    fn height_cells(mut self, cells: f32) -> Self {
        self.size.height = Dimension::length(cells);
        self
    }

    fn width_percent(mut self, pct: f32) -> Self {
        self.size.width = Dimension::percent(pct);
        self
    }

    fn height_percent(mut self, pct: f32) -> Self {
        self.size.height = Dimension::percent(pct);
        self
    }

    fn padding_all(mut self, cells: f32) -> Self {
        self.padding = taffy::geometry::Rect {
            left: LengthPercentage::length(cells),
            right: LengthPercentage::length(cells),
            top: LengthPercentage::length(cells),
            bottom: LengthPercentage::length(cells),
        };
        self
    }

    fn margin_all(mut self, cells: f32) -> Self {
        self.margin = taffy::geometry::Rect {
            left: LengthPercentageAuto::length(cells),
            right: LengthPercentageAuto::length(cells),
            top: LengthPercentageAuto::length(cells),
            bottom: LengthPercentageAuto::length(cells),
        };
        self
    }

    fn center(mut self) -> Self {
        self.align_items = Some(AlignItems::CENTER);
        self.justify_content = Some(JustifyContent::CENTER);
        self
    }

    fn safe_center(mut self) -> Self {
        self.align_items = Some(AlignItems::SAFE_CENTER);
        self.justify_content = Some(JustifyContent::SAFE_CENTER);
        self
    }

    fn display_grid(mut self) -> Self {
        self.display = Display::Grid;
        self
    }

    fn grid_columns(mut self, count: u16) -> Self {
        self.display = Display::Grid;
        self.grid_template_columns = vec![fr(1.0); count as usize];
        self
    }

    fn grid_rows(mut self, count: u16) -> Self {
        self.display = Display::Grid;
        self.grid_template_rows = vec![fr(1.0); count as usize];
        self
    }

    fn grid_template_columns(mut self, tracks: Vec<TrackSizingFunction>) -> Self {
        self.display = Display::Grid;
        self.grid_template_columns = tracks.into_iter().map(Into::into).collect();
        self
    }

    fn grid_template_rows(mut self, tracks: Vec<TrackSizingFunction>) -> Self {
        self.display = Display::Grid;
        self.grid_template_rows = tracks.into_iter().map(Into::into).collect();
        self
    }

    fn grid_col_span(mut self, span_count: u16) -> Self {
        self.grid_column = span(span_count);
        self
    }

    fn grid_row_span(mut self, span_count: u16) -> Self {
        self.grid_row = span(span_count);
        self
    }

    fn grid_cell(mut self, row: i16, col: i16) -> Self {
        self.grid_row = line(row);
        self.grid_column = line(col);
        self
    }

    fn gap_cells(mut self, w: f32, h: f32) -> Self {
        self.gap = Size {
            width: LengthPercentage::length(w),
            height: LengthPercentage::length(h),
        };
        self
    }

    fn min_content_width(mut self) -> Self {
        self.size.width = Dimension::min_content();
        self
    }

    fn min_content_height(mut self) -> Self {
        self.size.height = Dimension::min_content();
        self
    }

    fn min_content(self) -> Self {
        self.min_content_width().min_content_height()
    }

    fn max_content_width(mut self) -> Self {
        self.size.width = Dimension::max_content();
        self
    }

    fn max_content_height(mut self) -> Self {
        self.size.height = Dimension::max_content();
        self
    }

    fn max_content(self) -> Self {
        self.max_content_width().max_content_height()
    }

    fn auto_width(mut self) -> Self {
        self.size.width = Dimension::auto();
        self
    }

    fn auto_height(mut self) -> Self {
        self.size.height = Dimension::auto();
        self
    }

    fn auto_size(self) -> Self {
        self.auto_width().auto_height()
    }
}
