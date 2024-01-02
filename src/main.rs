mod search;
mod theme;

use std::cell::RefCell;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::slice::Iter;
use std::env;
use std::path::PathBuf;

use emoji::Emoji;
use iced::alignment::Horizontal;
use iced::mouse::Button;
use iced::widget::text_input::{Id, Icon};
use iced::{Application, Settings, Element, Subscription, executor, Theme, Command, window, keyboard, Event, subscription, Renderer, Length, Font};
use iced::widget::{column,button, text_input, container, row, scrollable, text, responsive};
use serde::{Deserialize, Serialize};
use search::{SearchEngine, TantivySearch};
use theme::RoundedTheme;

// Values that could be useful to be configured
mod conf {
    pub const EMOJI_SIZE: u16 = 30;
    pub const SPACING: u16 = 6;
    pub const EMOJI_PER_LINE: u16 = 8;
    pub const EMOJI_FONT_SIZE: u16 = 16;
    pub const EMOJI_LINE_HEIGHT: f32 = 0.93;
    pub const MAX_HISTORY_SIZE: usize = 80;
}

// Application's constants
const MAIN_PADDING: u16 = 10;
const GOLDEN_RATIO: f32 = 1.618034;
const SCROLLBAR_PADDING: u16 = 12;
const EMOJI_FONT: Font = Font::with_name("Noto Color Emoji");

fn get_conf_dir() -> PathBuf {
    PathBuf::from(env::var("XDG_CONFIG_HOME").unwrap_or(
    format!("{}/.var/app/com.sheosi.bmoji/config",env::var("HOME").unwrap())
   ))
}

fn make_conf_dir() {
    std::fs::create_dir_all(get_conf_dir().join("bmoji")).unwrap()
}

fn get_options_path() -> PathBuf {
   println!("{:?}",env::var("XDG_CONFIG_HOME"));
   get_conf_dir().join("bmoji/options.json")
}

fn main() -> iced::Result {
    let width = (conf::EMOJI_SIZE+conf::SPACING)*conf::EMOJI_PER_LINE+MAIN_PADDING*2+SCROLLBAR_PADDING;
    let height = ((width as f32)/GOLDEN_RATIO).ceil() as u32;
    
    Bmoji::run(Settings {
        antialiasing: true,
        default_font: Font::with_name("Inter"),
        window: window::Settings {
            decorations: false,
            resizable: false,
            size: (width as u32, height),
            platform_specific: window::PlatformSpecific {
                application_id: "com.sheosi.bmoji".to_string()
            },
            icon: window::icon::from_file_data(include_bytes!("../flatpak/Icon-small.webp"),Some(image::ImageFormat::WebP)).ok(),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct BmojiOptions {
    #[serde(default)]
    history: EmojiHistory
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct EmojiHistory (Vec<String>);

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
        self.0.iter().map(|g|emoji::lookup_by_glyph::lookup(g)).filter(Option::is_some).map(Option::unwrap).collect()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}


impl BmojiOptions {
    fn load() -> Self {
        if let Ok(options_file) = File::open(get_options_path()){
            let reader = BufReader::new(options_file);
            serde_json::from_reader(reader).unwrap()    
        }
        else {
            BmojiOptions::default()
        }
    }

    fn save(&self) {
        make_conf_dir();
        let options_file = File::create(get_options_path()).unwrap();
        let writer = BufWriter::new(options_file);
        let options_with_lim_history = BmojiOptions{
            history: EmojiHistory(self.history.iter().take(conf::MAX_HISTORY_SIZE).cloned().collect())
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
    TravelAndPlaces
}

struct Bmoji {
    has_been_interacted: bool,
    search_query: String,
    variant_picker: Option<VariantPicker>,
    category: EmojiCategory,
    first_emoji: RefCell<Option<&'static Emoji>>,
    search_input_id: Id,
    options: BmojiOptions,
    search_eng: TantivySearch
}

struct VariantPicker {
    emoji: &'static Emoji
}

#[derive(Debug, Clone)]
enum BmojiMessage {
    Search(String),
    OnSearchEnter,
    Glyph(&'static str),
    ShowGlyphVariants(&'static Emoji),
    Event(Event),
    CategoryChanged(EmojiCategory)
}

fn emoji_button<'a>(glyph: &'static str, has_variants: bool) -> iced::widget::Button<'a, BmojiMessage, Renderer<theme::RoundedTheme>> {
    button(
        text(glyph)
        .size(conf::EMOJI_FONT_SIZE)
        .line_height(conf::EMOJI_LINE_HEIGHT)
        .horizontal_alignment(Horizontal::Center)
        .font(EMOJI_FONT)
    )
    .height(conf::EMOJI_SIZE)
    .width(conf::EMOJI_SIZE)
    .style(if has_variants {theme::ButtonStyle::Emoji} else {theme::ButtonStyle::Plain})
}

fn grid_row<'a>(emoji_row: &[&'static Emoji]) -> Element<'a, BmojiMessage, Renderer<theme::RoundedTheme>>  {
    let button_row = 
    emoji_row.iter().map(|emoji_data| {
            emoji_button(emoji_data.glyph, !emoji_data.variants.is_empty()).on_press(
                if emoji_data.variants.is_empty() {
                    BmojiMessage::Glyph(emoji_data.glyph)
                }
                else {
                    BmojiMessage::ShowGlyphVariants(emoji_data)
                }
            ).into()
        }
    ).collect::<Vec<_>>();
    row(button_row).spacing(conf::SPACING).into()
}

impl Bmoji {
    fn grid_of(&self, elements: Vec<&'static Emoji>) -> Element<'_, BmojiMessage, Renderer<RoundedTheme>> {
        responsive(move |size|{
            let max_per_row = (size.width/((conf::EMOJI_SIZE + conf::SPACING)as f32)).floor() as usize;
            let rows = elements
                .chunks(max_per_row)
                .map(grid_row).collect::<Vec<_>>();

            let emoji_grid = column(rows).spacing(conf::SPACING);
            scrollable(emoji_grid)
            .width(Length::Fill).into()
        }).into()
    }

    fn copy_and_quit(&mut self, glyph: &'static str) -> Command<BmojiMessage> {
        self.options.history.add(glyph.to_string());
        Command::batch([iced::clipboard::write(glyph.to_string()), self.save_and_quit()])
    }
    
    fn save_and_quit(&self) -> Command<BmojiMessage> {
        self.options.save();
        window::close()
    }
}

impl Application for Bmoji {
    type Message = BmojiMessage;
    type Theme = theme::RoundedTheme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let langs = get_langs();
        let langs_ref = langs.iter().map(|l|l.as_str()).collect::<Vec<&str>>();
    
        let search_eng = search::TantivySearch::new(&langs_ref);
    
        let options = BmojiOptions::load() ;
        let search_input_id = Id::unique();
        (Self {
            has_been_interacted: false,
            search_query: String::new(),
            variant_picker: None,
            category: if options.history.is_empty() {EmojiCategory::SmileysAndEmotion} else {EmojiCategory::History},
            first_emoji:  RefCell::new(None),
            search_input_id: search_input_id.clone(),
            options,
            search_eng
        }, iced::widget::text_input::focus(search_input_id))
    }

    fn title(&self) -> String {
        "Emojis".to_string()
    }

    fn theme(&self) -> RoundedTheme {
        let iced_theme = match dconf_rs::get_string("/org/gnome/desktop/interface/color-scheme").unwrap_or("default".to_string()).as_str() {
            "prefer-dark" => Theme::Dark,
            _ => Theme::Light
        };
        let accent_color = iced_theme.extended_palette().primary.strong.color;
        theme::RoundedTheme{internal: iced_theme, accent_color}

    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message{
            BmojiMessage::Search(query) => {
                self.search_query = query;
                self.variant_picker = None;
                self.has_been_interacted = true;
                iced::widget::text_input::focus(self.search_input_id.clone())
            },
            BmojiMessage::Glyph(glyph) => {
                self.copy_and_quit(glyph)
                
            },
            BmojiMessage::ShowGlyphVariants(emoji) => {
                self.has_been_interacted = true;
                self.variant_picker = Some(VariantPicker {emoji});
                Command::none()
            },
            BmojiMessage::CategoryChanged(category) => {
                self.category = category;
                self.variant_picker = None;
                self.has_been_interacted = true;
               self.search_query = String::new(); iced::widget::text_input::focus(self.search_input_id.clone())
            },
            BmojiMessage::Event(Event::Keyboard(keyboard::Event::KeyReleased { key_code: keyboard::KeyCode::Escape, modifiers: _ })) => {
                self.save_and_quit()
            },
            BmojiMessage::OnSearchEnter | BmojiMessage::Event(Event::Keyboard(keyboard::Event::KeyReleased { key_code: keyboard::KeyCode::Enter, modifiers: _ })) => {
                // Needed so that the borrow is dropped and we don't have two borrows at the same time
                let fm = self.first_emoji.borrow().clone(); 
                self.has_been_interacted = true;
                if let Some(first_emoji) = fm {
                    if first_emoji.variants.is_empty() {
                        self.copy_and_quit(first_emoji.glyph)
                    }
                    else {
                        self.variant_picker = Some(VariantPicker { emoji: first_emoji });
                        Command::none()
                    }
                }
                else {
                    Command::none()
                }
            },
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::ButtonPressed(Button::Left))) => {
                self.variant_picker = None;
                self.has_been_interacted = true;
                iced::widget::text_input::focus(self.search_input_id.clone())
            },
            BmojiMessage::Event(Event::Window(window::Event::Focused)) => {
                Command::none()
            },
            BmojiMessage::Event(Event::Window(window::Event::Unfocused)) => {
                if self.has_been_interacted {self.save_and_quit()}
                else {Command::none()}
            },
            // Avoid treating them as interactions
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::CursorMoved{..})) | BmojiMessage::Event(Event::Keyboard(iced::keyboard::Event::ModifiersChanged(_))) => {
                Command::none()
            },
            BmojiMessage::Event(Event::Mouse(iced::mouse::Event::CursorEntered)) => {
                window::gain_focus()
            },
            BmojiMessage::Event(Event::Mouse(_)) => {
                self.has_been_interacted = true;
                Command::none()
            },
            BmojiMessage::Event(Event::Keyboard(_)) => {
                self.has_been_interacted = true;
                Command::none()
            },
            BmojiMessage::Event(Event::Touch(_)) => {
                self.has_been_interacted = true;
                Command::none()
            }
            BmojiMessage::Event(_) => Command::none()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        subscription::events().map(Self::Message::Event)
    }

    fn view(&self) -> Element<'_, Self::Message,Renderer<RoundedTheme>> {
        let inp_search = text_input("Search...",&self.search_query)
            .on_input(BmojiMessage::Search)
            .on_submit(BmojiMessage::OnSearchEnter)
            .id(self.search_input_id.clone())
            .icon(Icon { font: EMOJI_FONT, code_point: '🔎', size: Some(16.0), spacing: 10.0, side: text_input::Side::Left });
        let clear_search = 
            button("X")
            .on_press_maybe(if self.search_query.is_empty() {None} else {Some(BmojiMessage::Search(String::new()))})
            .width(32)
            .style(theme::ButtonStyle::ClearSearch);
        let search_row = row![inp_search, clear_search].spacing(7);
        
        fn emojis_category(cat:&str) -> Vec<&'static Emoji> {
            emoji::lookup_by_glyph::iter_emoji().filter(|e|e.group == cat && !e.is_variant).collect()
        }

        let body: Element<'_, BmojiMessage,Renderer<RoundedTheme>> = if let Some(variant_picker) = self.variant_picker.as_ref() {
            *self.first_emoji.borrow_mut() = Some(variant_picker.emoji.variants.first().unwrap());

            iced_aw::card(text(variant_picker.emoji.glyph).font(EMOJI_FONT), 
                container(row(variant_picker.emoji.variants.iter().map(
                    |v|emoji_button(v.glyph, false).on_press(BmojiMessage::Glyph(v.glyph)).into()
                ).collect::<Vec<_>>()).spacing(7)).height(Length::Fill)).close_size(conf::EMOJI_SIZE as f32).height(Length::Fill).into()
        } else {
            let emoji_list= if self.search_query.is_empty() {
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
                self.search_eng.search_emojis(&self.search_query, conf::EMOJI_PER_LINE*3)
            }.into_iter().filter(|_|true).collect::<Vec<_>>();
            *self.first_emoji.borrow_mut() = emoji_list.first().cloned();

            self.grid_of(emoji_list)
        };

        fn category<'a>(glyph: &'static str, current_cat: EmojiCategory, category: EmojiCategory) -> iced::widget::Button<'a, BmojiMessage, Renderer<theme::RoundedTheme>> {
            button(text(glyph).font(EMOJI_FONT)).on_press(BmojiMessage::CategoryChanged(category)).style(
                if current_cat == category {
                    theme::ButtonStyle::Category
                }
                else {
                    theme::ButtonStyle::Plain
                }
            )
        }

        let history_style = 
            if self.category == EmojiCategory::History {
                theme::ButtonStyle::Category
            }
            else {
                theme::ButtonStyle::Plain
            };
        
        let history_on_press = 
            if self.options.history.is_empty() {None}
            else {Some(BmojiMessage::CategoryChanged(EmojiCategory::History))};

        let categories = row!(
            button(text("🕑").font(EMOJI_FONT)).on_press_maybe(history_on_press).style(history_style),
            category("😃", self.category, EmojiCategory::SmileysAndEmotion),
            category("🧑", self.category, EmojiCategory::PeopleAndBody),
            category("⚽", self.category, EmojiCategory::Activities),
            category("🐻", self.category, EmojiCategory::AnimalsAndNature),
            category("🎌", self.category, EmojiCategory::Flags),
            category("🍔", self.category, EmojiCategory::FoodAndDrink),
            category("💡", self.category, EmojiCategory::Objects),
            category("💕", self.category, EmojiCategory::Symbols),
            category("🚀", self.category, EmojiCategory::TravelAndPlaces),
        ).spacing(0).width(Length::Fill);

        container(column![search_row, body, categories].spacing(8)).padding(MAIN_PADDING).into()
    }
}

fn get_langs() -> Vec<String> {
    use fluent_langneg::{convert_vec_str_to_langids, negotiate};
    use unic_langid::LanguageIdentifier;
    use emoji::ANNOTATION_LANGS_AVAILABLE;

    const DEFAULT_LANG:&str = "en";
    const UTF8_SUFFIX: &str=".UTF-8";
    
    let lang_str = std::env::var("LANG").map(
        |s|{
            if s.ends_with(UTF8_SUFFIX){
                s[0..s.len()-UTF8_SUFFIX.len()].to_string()
            }
            else {
                s
            }
        }
    ).unwrap_or(DEFAULT_LANG.to_string());

    let def_lang: LanguageIdentifier = LanguageIdentifier::from_parts("en".parse().expect("Devel error"), None, None, &[]);
    let lang: LanguageIdentifier = lang_str.parse().unwrap();
    let available = convert_vec_str_to_langids(ANNOTATION_LANGS_AVAILABLE).unwrap();

    let negotation = negotiate::negotiate_languages(
        &[lang], &available, Some(&def_lang), negotiate::NegotiationStrategy::Matching
    );

    [negotation.get(0).unwrap().to_string(), DEFAULT_LANG.to_string()].to_vec()
}
