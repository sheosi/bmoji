use emoji::{lookup_by_glyph::iter_emoji, Emoji};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::QueryParser,
    schema::{Field, Schema, Value, STORED, TEXT},
    DocAddress, Index, Score, Searcher, TantivyDocument,
};

pub trait SearchEngine {
    fn search_emojis(&self, emoji: &str, max_count: u32) -> Vec<&'static Emoji>;
}

pub struct DefSearch {
    lang: String,
}

impl DefSearch {
    pub fn new(lang: String) -> Self {
        DefSearch { lang }
    }
}

impl SearchEngine for DefSearch {
    fn search_emojis(&self, emoji: &str, _max_count: u32) -> Vec<&'static Emoji> {
        emoji::search::search_annotation_all(emoji)
    }
}

pub struct TantivySearch {
    searcher: Searcher,
    glyph: Field,
    query_parser: QueryParser,
}

struct DataPair {
    annotation: Field,
    glyph: Field,
}

impl TantivySearch {
    fn update_index(index: &Index, langs: &[&str], annotation: Field, glyph: Field) {
        let mut index_writer = index.writer(15_000_000).unwrap();

        for emoji in iter_emoji() {
            let mut full_annotation = String::new();
            for annotation in emoji.annotations {
                if langs.contains(&annotation.lang) {
                    full_annotation.push_str(&annotation.keywords.join(","));
                }
            }
            index_writer
                .add_document(doc!(
                    annotation => full_annotation,
                    glyph =>emoji.glyph
                ))
                .unwrap();
        }

        index_writer.commit().unwrap();
    }

    fn extract_fields(index: &Index) -> DataPair {
        let annotation = index.schema().find_field("annotation").unwrap().0;
        let glyph = index.schema().find_field("glyph").unwrap().0;
        DataPair { annotation, glyph }
    }

    pub fn new(langs: &[&str]) -> Self {
        const INDEX_PATH: &str = "./index";

        fn has_index(path: &str) -> bool {
            use tantivy::directory::error::OpenDirectoryError;
            match MmapDirectory::open(path) {
                Ok(mmapdir) => Index::exists(&mmapdir).unwrap(),
                Err(OpenDirectoryError::DoesNotExist(_)) => {
                    std::fs::create_dir_all(INDEX_PATH).unwrap();
                    false
                }
                Err(a) => panic!("Error while opening index dir: {a}"),
            }
        }

        let (index, annotation, glyph) = {
            if has_index(INDEX_PATH) {
                let index = Index::open_in_dir(INDEX_PATH).unwrap();
                let data_pair = Self::extract_fields(&index);
                (index, data_pair.annotation, data_pair.glyph)
            } else {
                let mut schema_builder = Schema::builder();
                let annotation = schema_builder.add_text_field("annotation", TEXT);
                let glyph = schema_builder.add_text_field("glyph", TEXT | STORED);
                let schema = schema_builder.build();

                let index = Index::create_in_dir(INDEX_PATH, schema.clone()).unwrap();

                (index, annotation, glyph)
            }
        };

        Self::update_index(&index, langs, annotation, glyph);
        let query_parser = QueryParser::for_index(&index, vec![annotation, glyph]);

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();

        Self {
            searcher,
            glyph,
            query_parser,
        }
    }
}

impl SearchEngine for TantivySearch {
    fn search_emojis(&self, emoji: &str, max_count: u32) -> Vec<&'static Emoji> {
        use emoji::lookup_by_glyph::lookup;

        let query = self.query_parser.parse_query(emoji).unwrap();

        let top_docs: Vec<(Score, DocAddress)> = self
            .searcher
            .search(&query, &TopDocs::with_limit(max_count as usize))
            .unwrap();

        top_docs
            .into_iter()
            .map(|(_, doc_address)| {
                let retrieved_doc: TantivyDocument = self.searcher.doc(doc_address).unwrap();
                let a = retrieved_doc
                    .get_first(self.glyph)
                    .unwrap()
                    .as_str()
                    .unwrap();
                lookup(a).unwrap()
            })
            .collect()
    }
}
