mod search;
mod theme;

use std::cell::RefCell;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::slice::Iter;
use std::str::FromStr;

use emoji::Emoji;
use iced::alignment::Horizontal;
use iced::keyboard::key::Named;
use iced::widget::operation::focus;
use iced::widget::text_input::Icon;
use iced::widget::{button, column, container, responsive, row, scrollable, text, text_input, Id};
use iced::{keyboard, window, Element, Event, Font, Length, Pixels, Renderer, Settings, Task};
use search::{SearchEngine, TantivySearch};
use serde::{Deserialize, Serialize};
use theme::RoundedTheme;

use crate::theme::ButtonStyle;

// Values that could be useful to be configured
mod conf {
    pub const EMOJI_SIZE: u32 = 30;
    pub const SPACING: u32 = 6;
    pub const EMOJI_PER_LINE: u32 = 8;
    pub const EMOJI_FONT_SIZE: u32 = 16;
    pub const EMOJI_LINE_HEIGHT: f32 = 0.93;
    pub const MAX_HISTORY_SIZE: usize = 80;
}

// Application's constants
const MAIN_PADDING: u32 = 10;
const GOLDEN_RATIO: f32 = 1.618034;
const SCROLLBAR_PADDING: u32 = 12;
const EMOJI_FONT: Font = Font::with_name("Noto Color Emoji");
const SEARCH_INPUT_ID: &str = "search";

fn get_conf_dir() -> PathBuf {
    PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or(format!(
        "{}/.var/app/com.sheosi.bmoji/config",
        env::var("HOME").unwrap()
    )))
}

fn make_conf_dir() {
    std::fs::create_dir_all(get_conf_dir().join("bmoji")).unwrap()
}

fn get_options_path() -> PathBuf {
    println!("{:?}", env::var("XDG_CONFIG_HOME"));
    get_conf_dir().join("bmoji/options.json")
}

fn main() -> iced::Result {
    let width = ((conf::EMOJI_SIZE + conf::SPACING) * conf::EMOJI_PER_LINE
        + MAIN_PADDING * 2
        + SCROLLBAR_PADDING) as f32;
    let height = ((width as f32) / GOLDEN_RATIO).ceil();

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
            application_id: "com.sheosi.bmoji".to_string(),
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
        if let Ok(options_file) = File::open(get_options_path()) {
            let reader = BufReader::new(options_file);
            serde_json::from_reader(reader).unwrap()
        } else {
            BmojiOptions::default()
        }
    }

    fn save(&self) {
        make_conf_dir();
        let options_file = File::create(get_options_path()).unwrap();
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
    Glyph(&'static str),
    ShowGlyphVariants(&'static Emoji),
    Event(Event),
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
            .font(EMOJI_FONT),
    )
    .height(conf::EMOJI_SIZE)
    .width(conf::EMOJI_SIZE)
    .class(ButtonStyle::Emoji)
}

fn grid_row<'a>(emoji_row: &[&'static Emoji]) -> Element<'a, BmojiMessage, RoundedTheme, Renderer> {
    let button_row = emoji_row
        .iter()
        .map(|emoji_data| {
            emoji_button(emoji_data.glyph, !emoji_data.variants.is_empty())
                .on_press(if emoji_data.variants.is_empty() {
                    BmojiMessage::Glyph(emoji_data.glyph)
                } else {
                    BmojiMessage::ShowGlyphVariants(emoji_data)
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

            let emoji_grid = column(rows).spacing(conf::SPACING);
            scrollable(emoji_grid).width(Length::Fill).into()
        })
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
        (Self::default(), focus(SEARCH_INPUT_ID))
    }

    fn update(&mut self, message: BmojiMessage) -> iced::Task<BmojiMessage> {
        match message {
            BmojiMessage::Search(query) => {
                self.search_query = query;
                self.variant_picker = None;
                self.has_been_interacted = true;
                iced::widget::operation::focus(self.search_input_id.clone())
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
                iced::widget::operation::focus(self.search_input_id.clone())
            }
            BmojiMessage::Event(Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(Named::Escape),
                modifiers: _,
                ..
            })) => self.save_and_quit(),
            BmojiMessage::OnSearchEnter
            | BmojiMessage::Event(Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(Named::Enter),
                modifiers: _,
                ..
            })) => {
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
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))) => {
                self.variant_picker = None;
                self.has_been_interacted = true;
                iced::widget::operation::focus(self.search_input_id.clone())
            }
            BmojiMessage::Event(Event::Window(window::Event::Focused)) => Task::none(),
            BmojiMessage::Event(Event::Window(window::Event::Unfocused)) => {
                if self.has_been_interacted {
                    self.save_and_quit()
                } else {
                    Task::none()
                }
            }
            // Avoid treating them as interactions
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::CursorMoved { .. }))
            | BmojiMessage::Event(Event::Keyboard(iced::keyboard::Event::ModifiersChanged(_))) => {
                Task::none()
            }
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::CursorEntered)) => {
                window::latest().and_then(window::gain_focus)
            }
            BmojiMessage::Event(Event::Mouse(_)) => {
                self.has_been_interacted = true;
                Task::none()
            }
            BmojiMessage::Event(Event::Keyboard(_)) => {
                self.has_been_interacted = true;
                Task::none()
            }
            BmojiMessage::Event(Event::Touch(_)) => {
                self.has_been_interacted = true;
                Task::none()
            }
            BmojiMessage::Event(_) => Task::none(),
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
            });
        let clear_search = button("X")
            .on_press_maybe(if self.search_query.is_empty() {
                None
            } else {
                Some(BmojiMessage::Search(String::new()))
            })
            .width(32)
            .class(ButtonStyle::ClearSearch);
        let search_row = row![inp_search, clear_search].spacing(7);

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

            self.grid_of(emoji_list)
        };

        fn category<'a>(
            glyph: &'static str,
            current_cat: EmojiCategory,
            category: EmojiCategory,
        ) -> iced::widget::Button<'a, BmojiMessage, RoundedTheme> {
            button(text(glyph).font(EMOJI_FONT))
                .on_press(BmojiMessage::CategoryChanged(category))
                .class(if current_cat == category {
                    theme::ButtonStyle::Category
                } else {
                    theme::ButtonStyle::Plain
                })
        }

        let history_style = if self.category == EmojiCategory::History {
            theme::ButtonStyle::Category
        } else {
            theme::ButtonStyle::Plain
        };

        let history_on_press = if self.options.history.is_empty() {
            None
        } else {
            Some(BmojiMessage::CategoryChanged(EmojiCategory::History))
        };

        let categories = row!(
            button(text("🕑").font(EMOJI_FONT))
                .on_press_maybe(history_on_press)
                .class(history_style),
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
        .width(Length::Fill);

        container(column![search_row, body, categories].spacing(8))
            .padding(MAIN_PADDING as u16)
            .into()
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
