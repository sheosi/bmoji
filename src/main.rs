mod search;
mod theme;

use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::slice::Iter;
use std::str::FromStr;
use std::sync::LazyLock;

use emoji::Emoji;
use iced::alignment::{self, Horizontal, Vertical};
use iced::widget::operation::focus;
use iced::widget::text_input::Icon;
use iced::widget::{
    button, column, container, responsive, row, scrollable, text, text_input, Container, Id,
};
use iced::{
    event, keyboard, window, Element, Font, Length, Pixels, Renderer, Settings, Subscription, Task,
};
use search::{SearchEngine, TantivySearch};
use serde::{Deserialize, Serialize};
use theme::RoundedTheme;

use crate::theme::{ButtonStyle, TextType};

// Values that could be useful to be configured
mod conf {
    pub const EMOJI_SIZE: u32 = 33;
    pub const SPACING: u32 = 6;
    pub const EMOJI_PER_LINE: u32 = 9;
    pub const EMOJI_FONT_SIZE: u32 = 23;
    pub const EMOJI_LINE_HEIGHT: f32 = 0.93;
    pub const MAX_HISTORY_SIZE: usize = 80;
    pub const CAT_EMOJI_FONT_SIZE: u32 = 23;
    pub const CAT_EMOJI_SIZE: u32 = 35;
}

// Application's constants
const VER_PADDING: u32 = 4;
const HOR_PADDING: u32 = 7;
const WINDOW_RATIO: f32 = 1.618034;
const SCROLLBAR_PADDING: u32 = 12;
const EMOJI_FONT: Font = Font::with_name("Noto Color Emoji");

fn get_conf_dir() -> PathBuf {
    PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or(format!(
        "{}/.var/app/io.github.sheosi.bmoji/config",
        env::var("HOME").unwrap()
    )))
}

fn make_conf_dir() {
    std::fs::create_dir_all(OPTIONS_PATH.parent().expect("")).expect("")
}

static OPTIONS_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| get_conf_dir().join("bmoji/options.json"));

fn main() -> iced::Result {
    let width = ((conf::EMOJI_SIZE + conf::SPACING) * conf::EMOJI_PER_LINE
        + VER_PADDING * 2
        + SCROLLBAR_PADDING) as f32;
    let height = ((width as f32) / WINDOW_RATIO).ceil();

    let app_settings = Settings {
        antialiasing: true,
        default_font: Font::with_name("Inter"),
        ..Default::default()
    };

    let window_settings = window::Settings {
        decorations: false,
        resizable: false,
        size: iced::Size { width, height },
        platform_specific: window::settings::PlatformSpecific {
            application_id: "io.github.sheosi.bmoji".to_string(),
            override_redirect: false,
        },
        icon: window::icon::from_file_data(
            include_bytes!("../flatpak/Icon-small.webp"),
            Some(image::ImageFormat::WebP),
        )
        .ok(),
        ..Default::default()
    };

    iced::application(Bmoji::new, Bmoji::update, Bmoji::view)
        .subscription(Bmoji::subscription)
        .settings(app_settings)
        .window(window_settings)
        .run()
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct BmojiOptions {
    #[serde(default)]
    history: EmojiHistory,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct EmojiHistory(Vec<String>);

impl EmojiHistory {
    fn add(&mut self, glyph: String) {
        if let Some(pos) = self.0.iter().position(|s| s == &glyph) {
            self.0.remove(pos);
        }

        self.0.insert(0, glyph);
    }

    fn iter(&self) -> Iter<'_, String> {
        self.0.iter()
    }

    fn emojis(&self) -> Vec<&'static Emoji> {
        self.0
            .iter()
            .map(|g| emoji::lookup_by_glyph::lookup(g))
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl BmojiOptions {
    fn load() -> Self {
        if let Ok(options_file) = File::open(OPTIONS_PATH.as_path()) {
            let reader = BufReader::new(options_file);
            serde_json::from_reader(reader).unwrap()
        } else {
            BmojiOptions::default()
        }
    }

    fn save(&self) {
        make_conf_dir();
        let options_file = File::create(OPTIONS_PATH.as_path()).unwrap();
        let writer = BufWriter::new(options_file);
        let options_with_lim_history = BmojiOptions {
            history: EmojiHistory(
                self.history
                    .iter()
                    .take(conf::MAX_HISTORY_SIZE)
                    .cloned()
                    .collect(),
            ),
        };
        serde_json::to_writer(writer, &options_with_lim_history).unwrap();
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum EmojiCategory {
    History,
    Activities,
    AnimalsAndNature,
    Flags,
    FoodAndDrink,
    Objects,
    PeopleAndBody,
    SmileysAndEmotion,
    Symbols,
    TravelAndPlaces,
}

struct Bmoji {
    has_been_interacted: bool,
    search_query: String,
    variant_picker: Option<VariantPicker>,
    category: EmojiCategory,
    first_emoji: RefCell<Option<&'static Emoji>>,
    search_input_id: Id,
    options: BmojiOptions,
    search_eng: TantivySearch,
}

impl Default for Bmoji {
    fn default() -> Self {
        let langs = get_langs();
        let langs_ref = langs.iter().map(|l| l.as_str()).collect::<Vec<&str>>();

        let search_eng = search::TantivySearch::new(&langs_ref);

        let options = BmojiOptions::load();
        let search_input_id = Id::unique();
        Self {
            has_been_interacted: false,
            search_query: String::new(),
            variant_picker: None,
            category: if options.history.is_empty() {
                EmojiCategory::SmileysAndEmotion
            } else {
                EmojiCategory::History
            },
            first_emoji: RefCell::new(None),
            search_input_id: search_input_id.clone(),
            options,
            search_eng,
        }
    }
}

struct VariantPicker {
    emoji: &'static Emoji,
}

#[derive(Debug, Clone)]
enum BmojiMessage {
    Search(String),
    OnSearchEnter,
    Interaction,
    Quit,
    OnUnfocused,
    GainFocus,
    SimpleInteraction,
    Glyph(&'static str),
    ShowGlyphVariants(&'static Emoji),
    CategoryChanged(EmojiCategory),
}

fn emoji_button<'a>(
    glyph: &'static str,
    has_variants: bool,
) -> iced::widget::Button<'a, BmojiMessage, RoundedTheme> {
    button(
        text(glyph)
            .size(conf::EMOJI_FONT_SIZE)
            .line_height(conf::EMOJI_LINE_HEIGHT)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .font(EMOJI_FONT),
    )
    .height(conf::EMOJI_SIZE)
    .width(conf::EMOJI_SIZE)
    .padding(7.5)
    .class(if has_variants {
        ButtonStyle::Category
    } else {
        ButtonStyle::Emoji
    })
}

fn grid_row<'a>(emoji_row: &[&'static Emoji]) -> Element<'a, BmojiMessage, RoundedTheme, Renderer> {
    let button_row = emoji_row
        .iter()
        .map(|emoji_data| {
            let is_variant = emoji_data.variants.len() > 1;
            emoji_button(emoji_data.glyph, is_variant)
                .on_press(if is_variant {
                    BmojiMessage::ShowGlyphVariants(emoji_data)
                } else {
                    BmojiMessage::Glyph(emoji_data.glyph)
                })
                .into()
        })
        .collect::<Vec<_>>();
    row(button_row).spacing(conf::SPACING).into()
}

impl Bmoji {
    fn grid_of(&self, elements: Vec<&'static Emoji>) -> Element<'_, BmojiMessage, RoundedTheme> {
        responsive(move |size| {
            let max_per_row =
                (size.width / ((conf::EMOJI_SIZE + conf::SPACING) as f32)).floor() as usize;
            let rows = elements
                .chunks(max_per_row)
                .map(grid_row)
                .collect::<Vec<_>>();

            let emoji_grid = column(rows).spacing(conf::SPACING).padding(0);
            scrollable(emoji_grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
        .height(Length::Fill)
        .into()
    }

    fn copy_and_quit(&mut self, glyph: &'static str) -> Task<BmojiMessage> {
        self.options.history.add(glyph.to_string());
        Task::batch([
            iced::clipboard::write(glyph.to_string()),
            self.save_and_quit(),
        ])
    }

    fn save_and_quit(&self) -> Task<BmojiMessage> {
        self.options.save();
        window::latest().and_then(window::close)
    }
}

impl Bmoji {
    fn new() -> (Self, Task<BmojiMessage>) {
        let res = Self::default();
        let id = res.search_input_id.clone();
        (res, focus(id))
    }

    fn update(&mut self, message: BmojiMessage) -> iced::Task<BmojiMessage> {
        match message {
            BmojiMessage::Search(query) => {
                self.search_query = query;
                self.variant_picker = None;
                self.has_been_interacted = true;
                focus(self.search_input_id.clone())
            }
            BmojiMessage::Glyph(glyph) => self.copy_and_quit(glyph),
            BmojiMessage::ShowGlyphVariants(emoji) => {
                self.has_been_interacted = true;
                self.variant_picker = Some(VariantPicker { emoji });
                Task::none()
            }
            BmojiMessage::CategoryChanged(category) => {
                self.category = category;
                self.variant_picker = None;
                self.has_been_interacted = true;
                self.search_query = String::new();
                focus(self.search_input_id.clone())
            }
            BmojiMessage::Quit => self.save_and_quit(),
            BmojiMessage::OnSearchEnter => {
                // Needed so that the borrow is dropped and we don't have two borrows at the same time
                let fm = self.first_emoji.borrow().clone();
                self.has_been_interacted = true;
                if let Some(first_emoji) = fm {
                    if first_emoji.variants.is_empty() {
                        self.copy_and_quit(first_emoji.glyph)
                    } else {
                        self.variant_picker = Some(VariantPicker { emoji: first_emoji });
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            BmojiMessage::Interaction => {
                self.variant_picker = None;
                self.has_been_interacted = true;
                focus(self.search_input_id.clone())
            }
            BmojiMessage::OnUnfocused => {
                if self.has_been_interacted {
                    self.save_and_quit()
                } else {
                    Task::none()
                }
            }
            BmojiMessage::SimpleInteraction => {
                self.has_been_interacted = true;
                Task::none()
            }
            BmojiMessage::GainFocus => window::latest().and_then(window::gain_focus),
        }
    }

    fn view(&self) -> Element<'_, BmojiMessage, RoundedTheme> {
        let inp_search = text_input("Search...", &self.search_query)
            .on_input(BmojiMessage::Search)
            .on_submit(BmojiMessage::OnSearchEnter)
            .id(self.search_input_id.clone())
            .icon(Icon {
                font: EMOJI_FONT,
                code_point: '🔎',
                size: Some(Pixels(16.0)),
                spacing: 10.0,
                side: text_input::Side::Left,
            })
            .line_height(1.2)
            .padding(6);

        let clear_search = button(
            text("X")
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center),
        )
        .on_press(if self.search_query.is_empty() {
            BmojiMessage::Quit
        } else {
            BmojiMessage::Search(String::new())
        })
        .height(32)
        .width(32)
        .class(ButtonStyle::ClearSearch);
        let search_row = row![inp_search, clear_search].spacing(7).padding(9);

        fn emojis_category(cat: &str) -> Vec<&'static Emoji> {
            emoji::lookup_by_glyph::iter_emoji()
                .filter(|e| e.group == cat && !e.is_variant)
                .collect()
        }

        let body: Element<'_, BmojiMessage, RoundedTheme> = if let Some(variant_picker) =
            self.variant_picker.as_ref()
        {
            *self.first_emoji.borrow_mut() = Some(variant_picker.emoji.variants.first().unwrap());

            iced_aw::card(
                text(variant_picker.emoji.glyph).font(EMOJI_FONT),
                container(
                    row(variant_picker
                        .emoji
                        .variants
                        .iter()
                        .map(|v| {
                            emoji_button(v.glyph, false)
                                .on_press(BmojiMessage::Glyph(v.glyph))
                                .into()
                        })
                        .collect::<Vec<_>>())
                    .spacing(7),
                )
                .height(Length::Fill),
            )
            .close_size(conf::EMOJI_SIZE as f32)
            .height(Length::Fill)
            .into()
        } else {
            let emoji_list = if self.search_query.is_empty() {
                match self.category {
                    EmojiCategory::History => self.options.history.emojis(),
                    EmojiCategory::Activities => emojis_category("Activities"),
                    EmojiCategory::AnimalsAndNature => emojis_category("Animals & Nature"),
                    EmojiCategory::Flags => emojis_category("Flags"),
                    EmojiCategory::FoodAndDrink => emojis_category("Food & Drink"),
                    EmojiCategory::Objects => emojis_category("Objects"),
                    EmojiCategory::PeopleAndBody => emojis_category("People & Body"),
                    EmojiCategory::SmileysAndEmotion => emojis_category("Smileys & Emotion"),
                    EmojiCategory::Symbols => emojis_category("Symbols"),
                    EmojiCategory::TravelAndPlaces => emojis_category("Travel & Places"),
                }
            } else {
                self.search_eng
                    .search_emojis(&self.search_query, conf::EMOJI_PER_LINE * 3)
            }
            .into_iter()
            .filter(|_| true)
            .collect::<Vec<_>>();
            *self.first_emoji.borrow_mut() = emoji_list.first().cloned();

            if emoji_list.is_empty() {
                let msg = if self.search_query.is_empty() {
                    "Use emojis for them to appear here"
                } else {
                    "Nothing found"
                };

                let txt: Element<'_, BmojiMessage, RoundedTheme> = iced::widget::Text::new(msg)
                    .class(TextType::Disabled)
                    .into();

                Container::new(txt)
                    .align_x(alignment::Horizontal::Center)
                    .align_y(alignment::Vertical::Center)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                self.grid_of(emoji_list)
            }
        };

        fn category_btn<'a>(
            glyph: &'static str,
            current_cat: EmojiCategory,
            category: EmojiCategory,
        ) -> iced::widget::Button<'a, BmojiMessage, RoundedTheme> {
            button(
                text(glyph)
                    .font(EMOJI_FONT)
                    .size(conf::CAT_EMOJI_FONT_SIZE)
                    .align_x(alignment::Horizontal::Center)
                    .align_y(alignment::Vertical::Center),
            )
            .class(if current_cat == category {
                theme::ButtonStyle::Category
            } else {
                theme::ButtonStyle::Plain
            })
            .padding([3, 5])
            .width(conf::CAT_EMOJI_SIZE)
        }

        fn category<'a>(
            glyph: &'static str,
            current_cat: EmojiCategory,
            category: EmojiCategory,
        ) -> iced::widget::Button<'a, BmojiMessage, RoundedTheme> {
            category_btn(glyph, current_cat, category)
                .on_press(BmojiMessage::CategoryChanged(category))
        }

        let history_on_press = if self.options.history.is_empty() {
            None
        } else {
            Some(BmojiMessage::CategoryChanged(EmojiCategory::History))
        };

        let categories = row!(
            category_btn("🕑", self.category, EmojiCategory::History)
                .on_press_maybe(history_on_press),
            category("😃", self.category, EmojiCategory::SmileysAndEmotion),
            category("🧑", self.category, EmojiCategory::PeopleAndBody),
            category("⚽", self.category, EmojiCategory::Activities),
            category("🐻", self.category, EmojiCategory::AnimalsAndNature),
            category("🎌", self.category, EmojiCategory::Flags),
            category("🍔", self.category, EmojiCategory::FoodAndDrink),
            category("💡", self.category, EmojiCategory::Objects),
            category("💕", self.category, EmojiCategory::Symbols),
            category("🚀", self.category, EmojiCategory::TravelAndPlaces),
        )
        .spacing(0)
        .padding(0)
        .align_y(alignment::Vertical::Bottom)
        .width(Length::Fill)
        .height(30);

        container(column![search_row, body, categories].spacing(2))
            .padding([VER_PADDING as u16, HOR_PADDING as u16])
            .into()
    }

    fn subscription(&self) -> Subscription<BmojiMessage> {
        use iced::{mouse, Event};

        event::listen().filter_map(|event| match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(BmojiMessage::Interaction)
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: keyboard::Key::Named(key),
                repeat: false,
                ..
            }) => {
                use keyboard::key::Named;
                match key {
                    Named::Enter => Some(BmojiMessage::OnSearchEnter),
                    Named::Escape => Some(BmojiMessage::Quit),

                    _ => None,
                }
            }

            Event::Mouse(mouse::Event::CursorEntered) => Some(BmojiMessage::GainFocus),

            // Avoid treating them as interactions
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Keyboard(keyboard::Event::ModifiersChanged(_)) => None,

            // Most events are treated as simple interactions, showing the user has some interest
            Event::Window(window::Event::Unfocused) => Some(BmojiMessage::OnUnfocused),
            Event::InputMethod(_) | Event::Keyboard(_) | Event::Touch(_) | Event::Mouse(_) => {
                Some(BmojiMessage::SimpleInteraction)
            }
            _ => None,
        })
    }
}

fn get_langs() -> Vec<String> {
    use emoji::ANNOTATION_LANGS_AVAILABLE;
    use fluent_langneg::{convert_vec_str_to_langids, negotiate};

    const DEFAULT_LANG: &str = "en";
    const UTF8_SUFFIX: &str = ".UTF-8";

    let lang_str = std::env::var("LANG")
        .map(|s| {
            if s.ends_with(UTF8_SUFFIX) {
                s[0..s.len() - UTF8_SUFFIX.len()].to_string()
            } else {
                s
            }
        })
        .unwrap_or(DEFAULT_LANG.to_string());

    let def_lang: fluent_langneg::LanguageIdentifier =
        fluent_langneg::LanguageIdentifier::from_str("en").expect("Devel error");
    let lang: fluent_langneg::LanguageIdentifier = lang_str.parse().unwrap();
    let available = convert_vec_str_to_langids(ANNOTATION_LANGS_AVAILABLE).unwrap();

    let negotation = negotiate::negotiate_languages(
        &[lang],
        &available,
        Some(&def_lang),
        negotiate::NegotiationStrategy::Matching,
    );

    [
        negotation.get(0).unwrap().to_string(),
        DEFAULT_LANG.to_string(),
    ]
    .to_vec()
}
