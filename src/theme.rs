use iced::widget::{button, text_input, scrollable};
use iced::Color;

#[derive(Clone)]
pub struct RoundedTheme {
    pub internal: iced::Theme,
    pub accent_color: iced::Color
}


impl Default for RoundedTheme {
    fn default() -> Self {
        Self { internal: iced::Theme::default(), accent_color: iced::Theme::default().extended_palette().background.strong.color }
    }
}

#[derive(PartialEq)]
pub enum ButtonStyle {
    Category,
    Emoji,
    Plain,
    ClearSearch
}

impl Default for ButtonStyle {
    fn default() -> Self {
        ButtonStyle::Plain
    }
}


impl iced::widget::button::StyleSheet for RoundedTheme {
    type Style = ButtonStyle;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = self.internal.extended_palette();
        let (background, text_color) = match style {
            ButtonStyle::Category => (Some(self.accent_color), palette.success.base.text),
            ButtonStyle::ClearSearch|ButtonStyle::Emoji => (Some(palette.secondary.base.color),palette.secondary.base.text),
            ButtonStyle::Plain => (None,palette.background.base.text),
        };
        button::Appearance { 
            border_radius: 8.0.into(),
            background: background.map(iced::Background::Color)
            ,text_color,
            ..button::Appearance::default()
        }
    }



    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        if style == &ButtonStyle::ClearSearch {
            let palette = self.internal.extended_palette();
            button::Appearance {
                background: Some(iced::Background::from(palette.background.strong.color)),
                ..active
            }
        }
        else {
            active
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);
        if style == &ButtonStyle::ClearSearch {
            button::Appearance {
                shadow_offset: iced::Vector::default(),
                ..active
            }
        }
        else {
            active
        }
    }

    fn disabled(&self, style: &Self::Style) -> button::Appearance {
        let active = self.active(style);

        if style == &ButtonStyle::ClearSearch {
            button::Appearance {
                shadow_offset: iced::Vector::default(),
                background: active.background.map(|background| match background {
                    iced::Background::Color(color) => iced::Background::Color(iced::Color {
                        a: color.a * 0.5,
                        ..color
                    }),
                    iced::Background::Gradient(gradient) => {
                        iced::Background::Gradient(gradient.mul_alpha(0.5))
                    }
                }),
                text_color: iced::Color {
                    a: active.text_color.a * 0.5,
                    ..active.text_color
                },
                ..active
            }
        }
        else {
            active
        }
    }

}

impl iced::widget::text_input::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.internal.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 8.0.into(),
            border_width: 1.2,
            border_color: palette.background.strong.color,
            icon_color: palette.background.weak.text,
        }
    }

    fn hovered(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.internal.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 8.0.into(),
            border_width: 1.2,
            border_color: palette.background.base.text,
            icon_color: palette.background.weak.text,
        }
    }

    fn focused(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.internal.extended_palette();

        text_input::Appearance {
            background: palette.background.base.color.into(),
            border_radius: 8.0.into(),
            border_width: 1.2,
            border_color: self.accent_color,
            icon_color: palette.background.weak.text,
        }
    }

    fn placeholder_color(&self, _style: &Self::Style) -> iced::Color {
        self.internal.extended_palette().background.strong.color
    }

    fn value_color(&self, _style: &Self::Style) -> iced::Color {
        self.internal.extended_palette().background.base.text
    }

    fn disabled_color(&self, _style: &Self::Style) -> iced::Color {
        self.internal.extended_palette().background.strong.color
    }

    fn selection_color(&self, _style: &Self::Style) -> iced::Color {
        self.accent_color
    }

    fn disabled(&self, _style: &Self::Style) -> text_input::Appearance {
        let palette = self.internal.extended_palette();

        text_input::Appearance {
            background: palette.background.weak.color.into(),
            border_radius: 8.0.into(),
            border_width: 1.2,
            border_color: palette.background.strong.color,
            icon_color: palette.background.strong.color,
        }
    }
}

impl iced::widget::scrollable::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        let palette = self.internal.extended_palette();

        scrollable::Scrollbar {
            background: Some(palette.background.weak.color.into()),
            border_radius: 8.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: palette.background.strong.color,
                border_radius: 8.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
        if is_mouse_over_scrollbar {
            let palette = self.internal.extended_palette();

            scrollable::Scrollbar {
                background: Some(palette.background.weak.color.into()),
                border_radius: 8.0.into(),
                border_width: 1.0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: palette.primary.strong.color,
                    border_radius: 8.0.into(),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            }
        } else {
            self.active(style)
        }
            
    }

    fn dragging(&self, style: &Self::Style) -> scrollable::Scrollbar {
        self.hovered(style, true)
    }

    fn active_horizontal(&self, style: &Self::Style) -> scrollable::Scrollbar {
        self.active(style)
    }

    fn hovered_horizontal(
        &self,
        style: &Self::Style,
        is_mouse_over_scrollbar: bool,
    ) -> scrollable::Scrollbar {
            self.hovered(style, is_mouse_over_scrollbar)
    }

    fn dragging_horizontal(
        &self,
        style: &Self::Style,
    ) -> scrollable::Scrollbar {
        self.hovered_horizontal(style, true)
    }
}

impl iced::widget::text::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn appearance(&self, _style: Self::Style) -> iced::widget::text::Appearance {
        self.internal.appearance(iced::theme::Text::Default)
    }
} 

impl iced::application::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn appearance(&self, _style: &Self::Style) -> iced::application::Appearance {
        self.internal.appearance(&iced::theme::Application::Default)
    }
}

impl iced::widget::container::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        self.internal.appearance(&iced::theme::Container::Transparent)
    }
}

impl iced_aw::card::StyleSheet for RoundedTheme {
    type Style = RoundedTheme;

    fn active(&self, _style: &Self::Style) -> iced_aw::card::Appearance {
        self.internal.active(&iced_aw::style::card::CardStyles::Light)
    }
}
