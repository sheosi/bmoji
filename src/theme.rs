use iced::widget::{button, scrollable, text_input};
use iced::Color;

#[derive(Clone)]
pub struct RoundedTheme {
    pub internal: iced::Theme,
    pub mode: iced::theme::Mode,
    pub accent_color: iced::Color,
}

#[derive(PartialEq)]
pub enum ButtonStyle {
    Category,
    Emoji,
    Plain,
    ClearSearch,
}

impl From<button::StyleFn<'_, RoundedTheme>> for ButtonStyle {
    fn from(value: button::StyleFn<'_, RoundedTheme>) -> Self {
        ButtonStyle::Category
    }
}

impl Default for ButtonStyle {
    fn default() -> Self {
        ButtonStyle::Plain
    }
}

impl iced::widget::button::Catalog for RoundedTheme {
    type Class<'a> = ButtonStyle;

    fn default<'a>() -> Self::Class<'a> {
        ButtonStyle::Plain
    }

    fn style(&self, class: &Self::Class<'_>, status: button::Status) -> button::Style {
        fn active(
            theme: &iced::Theme,
            accent_color: iced::Color,
            style: &ButtonStyle,
        ) -> button::Style {
            let palette = theme.extended_palette();
            let (background, text_color) = match style {
                ButtonStyle::Category => (Some(accent_color), palette.success.base.text),
                ButtonStyle::ClearSearch | ButtonStyle::Emoji => (
                    Some(palette.secondary.base.color),
                    palette.secondary.base.text,
                ),
                ButtonStyle::Plain => (None, palette.background.base.text),
            };
            button::Style {
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                background: background.map(iced::Background::Color),
                text_color,
                ..button::Style::default()
            }
        }

        fn hovered(
            theme: &iced::Theme,
            accent_color: iced::Color,
            style: &ButtonStyle,
        ) -> button::Style {
            let active = active(theme, accent_color, style);
            if style == &ButtonStyle::ClearSearch {
                let palette = theme.extended_palette();
                button::Style {
                    background: Some(iced::Background::from(palette.background.strong.color)),
                    ..active
                }
            } else {
                active
            }
        }

        fn pressed(
            theme: &iced::Theme,
            accent_color: iced::Color,
            style: &ButtonStyle,
        ) -> button::Style {
            let active = active(theme, accent_color, style);
            if style == &ButtonStyle::ClearSearch {
                button::Style {
                    shadow: iced::Shadow {
                        offset: iced::Vector::default(),
                        ..iced::Shadow::default()
                    },
                    ..active
                }
            } else {
                active
            }
        }

        fn disabled(
            theme: &iced::Theme,
            accent_color: iced::Color,
            style: &ButtonStyle,
        ) -> button::Style {
            let active = active(theme, accent_color, style);

            if style == &ButtonStyle::ClearSearch {
                button::Style {
                    shadow: iced::Shadow {
                        offset: iced::Vector::default(),
                        ..iced::Shadow::default()
                    },
                    background: active.background.map(|background| match background {
                        iced::Background::Color(color) => iced::Background::Color(iced::Color {
                            a: color.a * 0.5,
                            ..color
                        }),
                        iced::Background::Gradient(gradient) => {
                            iced::Background::Gradient(gradient.scale_alpha(0.5))
                        }
                    }),
                    text_color: iced::Color {
                        a: active.text_color.a * 0.5,
                        ..active.text_color
                    },
                    ..active
                }
            } else {
                active
            }
        }

        match status {
            button::Status::Active => active(&self.internal, self.accent_color, class),
            button::Status::Pressed => pressed(&self.internal, self.accent_color, class),
            button::Status::Hovered => hovered(&self.internal, self.accent_color, class),
            button::Status::Disabled => disabled(&self.internal, self.accent_color, class),
        }
    }
}

impl iced::theme::Base for RoundedTheme {
    fn default(preference: iced::theme::Mode) -> Self {
        let ref_theme = if preference == iced::theme::Mode::Light {
            iced::Theme::Light
        } else {
            iced::Theme::Dark
        };

        Self {
            accent_color: ref_theme.extended_palette().background.strong.color,
            mode: preference,
            internal: ref_theme,
        }
    }

    fn mode(&self) -> iced::theme::Mode {
        self.mode
    }

    fn base(&self) -> iced::theme::Style {
        self.internal.base()
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        Some(self.internal.palette())
    }

    fn name(&self) -> &str {
        "RoundedTheme"
    }
}

impl iced::widget::text_input::Catalog for RoundedTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        use text_input::Status;

        fn active(theme: &iced::Theme) -> text_input::Style {
            let palette = theme.extended_palette();

            text_input::Style {
                background: palette.background.base.color.into(),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.2,
                    color: palette.background.strong.color,
                },
                icon: palette.background.weak.text,
                ..iced::widget::text_input::default(theme, text_input::Status::Active)
            }
        }

        fn hovered(theme: &iced::Theme) -> text_input::Style {
            let palette = theme.extended_palette();

            text_input::Style {
                background: palette.background.base.color.into(),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.2,
                    color: palette.background.base.text,
                },
                icon: palette.background.weak.text,
                ..iced::widget::text_input::default(theme, text_input::Status::Hovered)
            }
        }

        fn focused(theme: &iced::Theme, accent_color: Color) -> text_input::Style {
            let palette = theme.extended_palette();

            text_input::Style {
                background: palette.background.base.color.into(),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.2,
                    color: accent_color,
                },
                icon: palette.background.weak.text,
                ..iced::widget::text_input::default(
                    theme,
                    text_input::Status::Focused { is_hovered: false },
                )
            }
        }

        fn disabled(theme: &iced::Theme) -> text_input::Style {
            let palette = theme.extended_palette();

            text_input::Style {
                background: palette.background.weak.color.into(),
                border: iced::Border {
                    radius: 8.0.into(),
                    width: 1.2,
                    color: palette.background.strong.color,
                },
                icon: palette.background.strong.color,
                ..iced::widget::text_input::default(theme, text_input::Status::Disabled)
            }
        }

        match status {
            Status::Active => active(&self.internal),
            Status::Hovered => hovered(&self.internal),
            Status::Focused { is_hovered: _ } => focused(&self.internal, self.accent_color),
            Status::Disabled => disabled(&self.internal),
        }
    }
}

impl iced::widget::scrollable::Catalog for RoundedTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        fn make_scrollbar(
            theme: &iced::Theme,
            border_width: f32,
            status: scrollable::Status,
        ) -> scrollable::Style {
            let palette = theme.extended_palette();
            scrollable::Style {
                container: iced::widget::container::transparent(theme),
                vertical_rail: scrollable::Rail {
                    background: Some(palette.background.weak.color.into()),
                    border: iced::Border {
                        radius: 8.0.into(),
                        width: border_width,
                        color: Color::TRANSPARENT,
                    },
                    scroller: scrollable::Scroller {
                        background: iced::Background::Color(palette.background.weak.color),
                        border: iced::Border {
                            radius: 8.0.into(),
                            width: border_width,
                            ..Default::default()
                        },
                    },
                },
                ..scrollable::default(theme, status)
            }
        }

        use iced::widget::scrollable::Status;

        match status {
            Status::Hovered {
                is_horizontal_scrollbar_hovered: _,
                is_vertical_scrollbar_hovered: _,
                is_horizontal_scrollbar_disabled: _,
                is_vertical_scrollbar_disabled: _,
            }
            | Status::Dragged {
                is_horizontal_scrollbar_dragged: _,
                is_vertical_scrollbar_dragged: _,
                is_horizontal_scrollbar_disabled: _,
                is_vertical_scrollbar_disabled: _,
            } => make_scrollbar(&self.internal, 1.0, status),
            Status::Active {
                is_horizontal_scrollbar_disabled: _,
                is_vertical_scrollbar_disabled: _,
            } => make_scrollbar(&self.internal, 0.0, status),
        }
    }
}

impl iced::widget::text::Catalog for RoundedTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _: &Self::Class<'_>) -> iced::widget::text::Style {
        iced::widget::text::base(&self.internal)
    }
}

impl iced::widget::container::Catalog for RoundedTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _: &Self::Class<'_>) -> iced::widget::container::Style {
        iced::widget::container::transparent(&self.internal)
    }
}

impl iced_aw::card::Catalog for RoundedTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _: &Self::Class<'_>, status: iced_aw::card::Status) -> iced_aw::card::Style {
        iced_aw::style::card::light(&self.internal, status)
    }
}
