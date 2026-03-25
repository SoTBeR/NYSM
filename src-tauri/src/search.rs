//! Tantivy full-text search integration.
//!
//! Provides in-process indexing and querying over the movie corpus.
//! The index lives in `{app_data_dir}/nysm_index/` and is rebuilt from
//! SQLite at every startup.

use std::path::PathBuf;
use std::sync::Mutex;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, TextFieldIndexing, TextOptions, FAST, INDEXED, STORED};
use tantivy::schema::Value;
use tantivy::tokenizer::{Language, LowerCaser, SimpleTokenizer, Stemmer, TextAnalyzer};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::types::{AppError, Movie};

const INDEX_SUBDIR: &str = "nysm_index";

// --------------------------------------------------------------------------
// Schema
// --------------------------------------------------------------------------

pub struct MovieSchema {
    pub schema: Schema,
    pub id: Field,
    pub title: Field,
    pub description: Field,
    pub actors: Field,
    pub genres: Field,
    pub studios: Field,
    pub year: Field,
    pub duration_minutes: Field,
    pub director: Field,
}

impl MovieSchema {
    pub fn build() -> Self {
        let mut builder = Schema::builder();
        let id = builder.add_u64_field("id", STORED | INDEXED | FAST);
        let year = builder.add_u64_field("year", STORED | INDEXED | FAST);
        let duration_minutes = builder.add_u64_field("duration_minutes", STORED | FAST);

        let text_opts = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("russian")
                    .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let title = builder.add_text_field("title", text_opts.clone());
        let description = builder.add_text_field("description", text_opts.clone());
        let actors = builder.add_text_field("actors", text_opts.clone());
        let genres = builder.add_text_field("genres", text_opts.clone());
        let studios = builder.add_text_field("studios", text_opts.clone());
        let director = builder.add_text_field("director", text_opts);

        MovieSchema {
            schema: builder.build(),
            id,
            title,
            description,
            actors,
            genres,
            studios,
            year,
            duration_minutes,
            director,
        }
    }
}

// --------------------------------------------------------------------------
// SearchIndex
// --------------------------------------------------------------------------

pub struct SearchIndex {
    pub index: Index,
    pub reader: IndexReader,
    pub schema: MovieSchema,
}

impl SearchIndex {
    pub fn open_or_create(index_dir: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&index_dir)
            .map_err(|e| AppError::Index(format!("Cannot create index dir: {e}")))?;

        let schema = MovieSchema::build();
        let mmap_dir = tantivy::directory::MmapDirectory::open(&index_dir)
            .map_err(|e| AppError::Index(e.to_string()))?;

        let index = match Index::open_or_create(mmap_dir, schema.schema.clone()) {
            Ok(idx) => idx,
            Err(e) => {
                eprintln!("[NYSM] Tantivy index schema mismatch ({e}), recreating...");
                let _ = std::fs::remove_dir_all(&index_dir);
                std::fs::create_dir_all(&index_dir)
                    .map_err(|e2| AppError::Index(format!("Cannot recreate index dir: {e2}")))?;
                Index::create_in_dir(&index_dir, schema.schema.clone())
                    .map_err(|e2| AppError::Index(e2.to_string()))?
            }
        };

        // Register Russian stemmer — must match the tokenizer name used in the schema.
        // Registration must happen before any indexing or searching.
        let russian_analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::Russian))
            .build();
        index.tokenizers().register("russian", russian_analyzer);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e: tantivy::TantivyError| AppError::Index(e.to_string()))?;

        Ok(SearchIndex { index, reader, schema })
    }

    pub fn writer(&self) -> Result<IndexWriter, AppError> {
        self.index
            .writer(50_000_000)
            .map_err(|e| AppError::Index(e.to_string()))
    }
}

// --------------------------------------------------------------------------
// State
// --------------------------------------------------------------------------

pub struct SearchState(pub Mutex<Option<SearchIndex>>);

impl SearchState {
    pub fn new() -> Self {
        SearchState(Mutex::new(None))
    }
}

// --------------------------------------------------------------------------
// Init
// --------------------------------------------------------------------------

pub fn init_index(state: &SearchState, app_data_dir: PathBuf) -> Result<(), AppError> {
    let index_path = app_data_dir.join(INDEX_SUBDIR);
    let search_index = SearchIndex::open_or_create(index_path)?;
    let mut guard = state
        .0
        .lock()
        .map_err(|_| AppError::Index("Lock poisoned".into()))?;
    *guard = Some(search_index);
    Ok(())
}

// --------------------------------------------------------------------------
// Index movies
// --------------------------------------------------------------------------

pub fn index_movies_internal(
    state: &SearchState,
    movies: &[Movie],
    clear_first: bool,
) -> Result<(), AppError> {
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::Index("Lock poisoned".into()))?;
    let idx = guard
        .as_ref()
        .ok_or_else(|| AppError::Index("Index not initialised".into()))?;
    let mut writer = idx.writer()?;

    if clear_first {
        writer
            .delete_all_documents()
            .map_err(|e| AppError::Index(e.to_string()))?;
    }

    let s = &idx.schema;
    for movie in movies {
        let mut doc = TantivyDocument::default();
        doc.add_u64(s.id, movie.id);
        doc.add_text(s.title, &movie.title);
        doc.add_text(s.description, &movie.description);
        doc.add_text(s.actors, movie.actors.join(", "));
        doc.add_text(s.genres, movie.genres.join(", "));
        doc.add_text(s.studios, movie.studios.join(", "));
        doc.add_u64(s.year, movie.year as u64);
        doc.add_u64(
            s.duration_minutes,
            movie.duration_minutes.unwrap_or(0) as u64,
        );
        doc.add_text(s.director, &movie.director);
        writer
            .add_document(doc)
            .map_err(|e| AppError::Index(e.to_string()))?;
    }

    writer
        .commit()
        .map_err(|e| AppError::Index(e.to_string()))?;
    // Explicit reload after commit to avoid OnCommitWithDelay race condition
    idx.reader
        .reload()
        .map_err(|e| AppError::Index(e.to_string()))?;
    Ok(())
}

// --------------------------------------------------------------------------
// Search
// --------------------------------------------------------------------------

pub fn search_movies_internal(
    state: &SearchState,
    query: &str,
    limit: usize,
) -> Result<Vec<u64>, AppError> {
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::Search("Lock poisoned".into()))?;
    let idx = guard
        .as_ref()
        .ok_or_else(|| AppError::Search("Index not initialised".into()))?;

    let searcher = idx.reader.searcher();
    let s = &idx.schema;

    let mut query_parser = QueryParser::for_index(
        &idx.index,
        vec![s.title, s.director, s.actors, s.genres, s.studios, s.description],
    );
    query_parser.set_field_boost(s.title, 4.0);
    query_parser.set_field_boost(s.director, 2.0);
    query_parser.set_field_boost(s.actors, 1.5);
    query_parser.set_field_boost(s.genres, 1.5);
    query_parser.set_field_boost(s.studios, 1.2);

    let query_lower = query.to_lowercase();
    let parsed = query_parser
        .parse_query(&query_lower)
        .or_else(|_| query_parser.parse_query(query))
        .map_err(|e| AppError::Search(format!("Query parse error: {e}")))?;

    let top_docs = searcher
        .search(&parsed, &TopDocs::with_limit(limit))
        .map_err(|e| AppError::Search(e.to_string()))?;

    let mut ids = Vec::with_capacity(top_docs.len());
    for (_score, doc_address) in top_docs {
        let doc: TantivyDocument = searcher
            .doc(doc_address)
            .map_err(|e| AppError::Search(e.to_string()))?;
        if let Some(id) = doc.get_first(s.id).and_then(|v| v.as_u64()) {
            ids.push(id);
        }
    }

    Ok(ids)
}

// --------------------------------------------------------------------------
// Tauri command
// --------------------------------------------------------------------------

/// Search movies using Tantivy and return full movie objects from DB.
///
/// Returns up to `limit` movies in Tantivy score order (best match first).
#[tauri::command]
pub async fn search_movies(
    query: String,
    limit: Option<usize>,
    search_state: tauri::State<'_, SearchState>,
    db_state: tauri::State<'_, crate::db::DbState>,
) -> Result<Vec<Movie>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let limit = limit.unwrap_or(20).min(100);

    // 1. Get matching IDs from Tantivy
    let ids = search_movies_internal(&search_state, &query, limit)?;
    if ids.is_empty() {
        return Ok(vec![]);
    }

    // 2. Fetch full movie data from DB for those IDs
    let all_movies = {
        let guard = db_state
            .0
            .lock()
            .map_err(|_| AppError::Database("Lock poisoned".into()))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| AppError::Database("DB not initialised".into()))?;
        crate::db::fetch_all_movies_sync(conn)?
    };

    // 3. Return movies in Tantivy score order
    let movie_map: std::collections::HashMap<u64, Movie> =
        all_movies.into_iter().map(|m| (m.id, m)).collect();

    let movies = ids
        .into_iter()
        .filter_map(|id| movie_map.get(&id).cloned())
        .collect();

    Ok(movies)
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_movies() -> Vec<Movie> {
        vec![
            Movie {
                id: 1,
                title: "Ирония судьбы".into(),
                description: "Новогодняя комедия про Женю Лукашина".into(),
                actors: vec!["Андрей Мягков".into(), "Барбара Брыльска".into()],
                genres: vec!["Комедия".into(), "Мелодрама".into()],
                studios: vec!["Мосфильм".into()],
                year: 1975,
                duration_minutes: Some(184),
                director: "Эльдар Рязанов".into(),
                rating: None,
            },
            Movie {
                id: 2,
                title: "Морозко".into(),
                description: "Сказка про добрую Настеньку и злую мачеху".into(),
                actors: vec!["Александр Хвыля".into(), "Наталья Седых".into()],
                genres: vec!["Сказка".into(), "Семейный".into()],
                studios: vec!["Мосфильм".into()],
                year: 1964,
                duration_minutes: Some(84),
                director: "Александр Роу".into(),
                rating: None,
            },
        ]
    }

    fn make_search_state(tmp: &TempDir) -> SearchState {
        let state = SearchState::new();
        init_index(&state, tmp.path().to_path_buf()).expect("init must succeed");
        state
    }

    #[test]
    fn index_and_search_returns_match() {
        let tmp = TempDir::new().unwrap();
        let state = make_search_state(&tmp);
        let movies = make_test_movies();

        index_movies_internal(&state, &movies, true).expect("index must succeed");

        let ids = search_movies_internal(&state, "рязанов", 10).expect("search must succeed");
        assert!(!ids.is_empty(), "should find Ironia sudby by director");
        assert!(ids.contains(&1), "movie id=1 must be in results");
    }

    #[test]
    fn search_cyrillic_returns_results() {
        let tmp = TempDir::new().unwrap();
        let state = make_search_state(&tmp);
        index_movies_internal(&state, &make_test_movies(), true).unwrap();

        let ids = search_movies_internal(&state, "Морозко", 10).unwrap();
        assert!(ids.contains(&2), "should find Morozko by title");
    }

    #[test]
    fn search_empty_query_is_handled_gracefully() {
        let tmp = TempDir::new().unwrap();
        let state = make_search_state(&tmp);
        index_movies_internal(&state, &make_test_movies(), true).unwrap();

        // An empty-ish query hits parse error path — ok to return empty or error
        // We just verify it doesn't panic
        let _ = search_movies_internal(&state, "", 10);
    }

    #[test]
    fn search_unrelated_query_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let state = make_search_state(&tmp);
        index_movies_internal(&state, &make_test_movies(), true).unwrap();

        let ids = search_movies_internal(&state, "qwertyuiop", 10).unwrap_or_default();
        assert!(ids.is_empty(), "no results expected for gibberish");
    }

    #[test]
    fn clear_first_removes_old_documents() {
        let tmp = TempDir::new().unwrap();
        let state = make_search_state(&tmp);
        let movies = make_test_movies();

        // Index both movies first
        index_movies_internal(&state, &movies, true).unwrap();

        // Re-index with only the second movie, clearing first
        index_movies_internal(&state, &movies[1..], true).unwrap();

        // Movie 1 must no longer appear
        let ids = search_movies_internal(&state, "рязанов", 10).unwrap_or_default();
        assert!(!ids.contains(&1), "movie id=1 should be gone after clear+reindex");
    }
}
