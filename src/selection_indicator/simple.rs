use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, DrawTargetExt, Point, Size},
    primitives::{Primitive, PrimitiveStyle, Rectangle, StyledDrawable},
    Drawable,
};

use crate::{
    adapters::invert::BinaryColorDrawTargetExt, selection_indicator::SelectionIndicator, MenuStyle,
};

pub struct SimpleSelectionIndicator {
    y_offset: i32,
}

impl SimpleSelectionIndicator {
    pub fn new() -> Self {
        Self { y_offset: 0 }
    }
}

impl SelectionIndicator for SimpleSelectionIndicator {
    type Color = BinaryColor;

    fn update_target(&mut self, y: i32) {
        self.y_offset = y;
    }

    fn offset(&self) -> i32 {
        self.y_offset
    }

    fn update(&mut self) {}

    fn draw<D>(
        &self,
        indicator_height: u32,
        screen_offset: i32,
        fill_width: u32,
        display: &mut D,
        items: &impl StyledDrawable<MenuStyle<Self::Color>, Color = Self::Color, Output = ()>,
        style: &MenuStyle<Self::Color>,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        Rectangle::new(
            Point::new(0, screen_offset),
            Size::new(fill_width.max(1), indicator_height),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display)?;

        let display_top_left = display.bounding_box().top_left;
        let display_size = display.bounding_box().size;

        let margin = Size::new(2, 0);

        let mut inverting = display.invert_area(&Rectangle::new(
            Point::new(0, screen_offset),
            Size::new(fill_width, indicator_height),
        ));
        items.draw_styled(
            style,
            &mut inverting.cropped(&Rectangle::new(
                display_top_left + margin,
                display_size - margin,
            )),
        )
    }
}
