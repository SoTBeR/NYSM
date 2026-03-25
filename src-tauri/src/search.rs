use std::path::PathBuf;
use std::sync::Mutex;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{
    Field, Schema, TextFieldIndexing, TextOptions, FAST, INDEXED, STORED,
};
use tantivy::schema::Value;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::types::{AppError, Movie, SearchResult};

/// Путь к директории индекса Tantivy внутри data dir приложения
const INDEX_SUBDIR: &str = "nysm_index";

// --------------------------------------------------------------------------
// Схема индекса
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

        // Числовые поля хранятся как FAST (для сортировки/фильтра) + STORED + INDEXED
        let id = builder.add_u64_field("id", STORED | INDEXED | FAST);
        let year = builder.add_u64_field("year", STORED | INDEXED | FAST);
        let duration_minutes = builder.add_u64_field("duration_minutes", STORED | FAST);

        // Текстовые поля для полнотекстового поиска
        let text_opts = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
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
// Глобальное состояние индекса
// --------------------------------------------------------------------------

pub struct SearchIndex {
    pub index: Index,
    pub reader: IndexReader,
    pub schema: MovieSchema,
}

impl SearchIndex {
    /// Открывает или создаёт индекс в указанной директории.
    ///
    /// Если на диске лежит индекс со старой схемой (например, после обновления
    /// кода), он автоматически удаляется и создаётся заново.
    /// Данные восстанавливаются из БД при следующем вызове `index_movies_internal`.
    pub fn open_or_create(index_dir: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&index_dir)
            .map_err(|e| AppError::Index(format!("Cannot create index dir: {e}")))?;

        let schema = MovieSchema::build();
        let mmap_dir = tantivy::directory::MmapDirectory::open(&index_dir)
            .map_err(|e| AppError::Index(e.to_string()))?;

        // open_or_create: открывает если схема совпадает, создаёт если индекса нет.
        // При несовпадении схем возвращает ошибку → стираем директорию и пересоздаём.
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

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e: tantivy::TantivyError| AppError::Index(e.to_string()))?;

        Ok(SearchIndex {
            index,
            reader,
            schema,
        })
    }

    /// Возвращает writer с буфером 50 МБ
    pub fn writer(&self) -> Result<IndexWriter, AppError> {
        self.index
            .writer(50_000_000)
            .map_err(|e| AppError::Index(e.to_string()))
    }
}

// --------------------------------------------------------------------------
// Tauri State
// --------------------------------------------------------------------------

/// Обёртка вокруг SearchIndex для хранения в Tauri state
pub struct SearchState(pub Mutex<Option<SearchIndex>>);

impl SearchState {
    pub fn new() -> Self {
        SearchState(Mutex::new(None))
    }
}

// --------------------------------------------------------------------------
// Индексирование фильмов
// --------------------------------------------------------------------------

/// Добавляет список фильмов в индекс.
/// Если `clear_first = true` — сначала очищает весь индекс.
pub fn index_movies_internal(
    state: &SearchState,
    movies: &[Movie],
    clear_first: bool,
) -> Result<(), AppError> {
    let guard = state.0.lock().map_err(|_| AppError::Index("Lock poisoned".into()))?;
    let idx = guard.as_ref().ok_or_else(|| AppError::Index("Index not initialised".into()))?;

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
        doc.add_u64(s.duration_minutes, movie.duration_minutes.unwrap_or(0) as u64);
        doc.add_text(s.director, &movie.director);

        writer
            .add_document(doc)
            .map_err(|e| AppError::Index(e.to_string()))?;
    }

    writer.commit().map_err(|e| AppError::Index(e.to_string()))?;
    // Явная перезагрузка reader чтобы новые документы были сразу видны
    idx.reader.reload().map_err(|e| AppError::Index(e.to_string()))?;
    Ok(())
}

// --------------------------------------------------------------------------
// Поиск
// --------------------------------------------------------------------------

/// Полнотекстовый поиск по индексу с поддержкой кириллицы.
///
/// Использует только QueryParser (без FuzzyTermQuery), так как FuzzyTermQuery
/// работает на уровне байтов UTF-8, что ломает нечёткий поиск по кирилллице
/// (каждый символ = 2 байта, edit-distance на уровне байтов не совпадает
/// с edit-distance на уровне кодовых точек Unicode).
///
/// Для нечёткости пользовательских запросов финальный ранжир выполняет AI.
pub fn fuzzy_search(
    state: &SearchState,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let guard = state.0.lock().map_err(|_| AppError::Search("Lock poisoned".into()))?;
    let idx = guard.as_ref().ok_or_else(|| AppError::Search("Index not initialised".into()))?;

    let searcher = idx.reader.searcher();
    let s = &idx.schema;

    // QueryParser с весами: title >> director > actors >= genres >= studios > description
    let mut query_parser = QueryParser::for_index(
        &idx.index,
        vec![s.title, s.director, s.actors, s.genres, s.studios, s.description],
    );
    query_parser.set_field_boost(s.title, 4.0);
    query_parser.set_field_boost(s.director, 2.0);
    query_parser.set_field_boost(s.actors, 1.5);
    query_parser.set_field_boost(s.genres, 1.5);
    query_parser.set_field_boost(s.studios, 1.2);

    // Строим запрос: пробуем оригинальный + нижний регистр
    let query_lower = query.to_lowercase();
    let parsed = query_parser
        .parse_query(&query_lower)
        .or_else(|_| query_parser.parse_query(query))
        .map_err(|e| AppError::Search(format!("Query parse error: {e}")))?;

    let top_docs = searcher
        .search(&parsed, &TopDocs::with_limit(limit))
        .map_err(|e| AppError::Search(e.to_string()))?;

    let mut results: Vec<SearchResult> = Vec::with_capacity(top_docs.len());
    for (score, doc_address) in top_docs {
        let retrieved: TantivyDocument = searcher
            .doc(doc_address)
            .map_err(|e| AppError::Search(e.to_string()))?;
        let movie = doc_to_movie(&retrieved, s)?;
        results.push(SearchResult { movie, score });
    }

    Ok(results)
}

/// Конвертирует TantivyDocument обратно в Movie
fn doc_to_movie(doc: &TantivyDocument, s: &MovieSchema) -> Result<Movie, AppError> {
    let get_text = |field: Field| -> String {
        doc.get_first(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let get_u64 = |field: Field| -> u64 {
        doc.get_first(field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };

    let split_csv = |raw: String| -> Vec<String> {
        if raw.is_empty() {
            vec![]
        } else {
            raw.split(", ").map(|s| s.trim().to_string()).collect()
        }
    };

    let duration = {
        let d = get_u64(s.duration_minutes);
        if d == 0 { None } else { Some(d as u32) }
    };

    Ok(Movie {
        id: get_u64(s.id),
        title: get_text(s.title),
        description: get_text(s.description),
        actors: split_csv(get_text(s.actors)),
        genres: split_csv(get_text(s.genres)),
        studios: split_csv(get_text(s.studios)),
        year: get_u64(s.year) as u32,
        duration_minutes: duration,
        director: get_text(s.director),
        rating: None,
    })
}

// --------------------------------------------------------------------------
// Tauri команды
// --------------------------------------------------------------------------

/// Инициализирует индекс. Вызывается из lib.rs при запуске.
pub fn init_index(state: &SearchState, app_data_dir: PathBuf) -> Result<(), AppError> {
    let index_path = app_data_dir.join(INDEX_SUBDIR);
    let search_index = SearchIndex::open_or_create(index_path)?;
    let mut guard = state.0.lock().map_err(|_| AppError::Index("Lock poisoned".into()))?;
    *guard = Some(search_index);
    Ok(())
}

#[tauri::command]
pub async fn search_movies(
    query: String,
    limit: Option<usize>,
    state: tauri::State<'_, SearchState>,
) -> Result<Vec<SearchResult>, AppError> {
    let limit = limit.unwrap_or(20).min(100);
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    fuzzy_search(&state, &query, limit)
}

#[tauri::command]
pub async fn index_movies(
    movies: Vec<Movie>,
    clear_first: Option<bool>,
    state: tauri::State<'_, SearchState>,
) -> Result<usize, AppError> {
    let count = movies.len();
    index_movies_internal(&state, &movies, clear_first.unwrap_or(false))?;
    Ok(count)
}

#[tauri::command]
pub async fn rebuild_index(
    movies: Vec<Movie>,
    state: tauri::State<'_, SearchState>,
) -> Result<usize, AppError> {
    let count = movies.len();
    index_movies_internal(&state, &movies, true)?;
    Ok(count)
}

// --------------------------------------------------------------------------
// Тесты
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Movie;
    use tempfile::TempDir;

    fn sample_movie(id: u64, title: &str) -> Movie {
        Movie {
            id,
            title: title.to_string(),
            description: format!("Описание для {title}"),
            actors: vec!["Актёр А".into(), "Актёр Б".into()],
            genres: vec!["Комедия".into()],
            studios: vec!["Мосфильм".into()],
            year: 1970 + id as u32,
            duration_minutes: Some(90 + id as u32),
            director: "Режиссёр".into(),
            rating: None,
        }
    }

    fn init_test_state(tmp: &TempDir) -> SearchState {
        let state = SearchState::new();
        init_index(&state, tmp.path().to_path_buf())
            .expect("test index should initialise");
        state
    }

    #[test]
    fn index_initialises_successfully() {
        let tmp = TempDir::new().unwrap();
        let state = SearchState::new();
        let result = init_index(&state, tmp.path().to_path_buf());
        assert!(result.is_ok(), "index init must succeed in a temp dir");
    }

    #[test]
    fn index_movies_returns_count() {
        let tmp = TempDir::new().unwrap();
        let state = init_test_state(&tmp);
        let movies = vec![sample_movie(1, "Ирония судьбы"), sample_movie(2, "Карнавальная ночь")];
        let result = index_movies_internal(&state, &movies, false);
        assert!(result.is_ok());
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let state = init_test_state(&tmp);
        let movies = vec![sample_movie(1, "Морозко")];
        index_movies_internal(&state, &movies, false).unwrap();

        let results = fuzzy_search(&state, "   ", 10).unwrap();
        assert!(results.is_empty(), "blank query must return empty results");
    }

    #[test]
    fn search_cyrillic_returns_results() {
        let tmp = TempDir::new().unwrap();
        let state = init_test_state(&tmp);
        let movies = vec![sample_movie(1, "Карнавальная ночь"), sample_movie(2, "Морозко")];
        index_movies_internal(&state, &movies, false).unwrap();

        // QueryParser (без FuzzyTermQuery) должен корректно искать по кириллице
        let result = fuzzy_search(&state, "карнавальная", 10);
        assert!(result.is_ok(), "Кириллический поиск не должен возвращать ошибку");
        // QueryParser с default токенизатором находит точные совпадения
        let results = result.unwrap();
        assert!(!results.is_empty(), "Кириллический запрос должен найти хотя бы один фильм");
        assert_eq!(results[0].movie.title, "Карнавальная ночь");
    }

    #[test]
    fn rebuild_index_replaces_previous_documents() {
        let tmp = TempDir::new().unwrap();
        let state = init_test_state(&tmp);

        // Индексируем старый набор
        let old = vec![sample_movie(1, "Старый фильм")];
        index_movies_internal(&state, &old, false).unwrap();

        // Полностью перестраиваем с новым набором
        let new = vec![sample_movie(2, "Новый фильм")];
        index_movies_internal(&state, &new, true).unwrap();

        // Старый фильм не должен найтись
        let old_results = fuzzy_search(&state, "Старый", 10).unwrap();
        assert!(
            old_results.is_empty(),
            "after rebuild old documents must be gone"
        );
    }

    #[test]
    fn movie_schema_all_fields_stored() {
        let schema = MovieSchema::build();
        // Все поля должны быть STORED — иначе doc_to_movie вернёт пустые значения
        let entry = schema.schema.get_field_entry(schema.title);
        assert!(
            entry.is_stored(),
            "title field must be stored for retrieval"
        );
        let entry = schema.schema.get_field_entry(schema.id);
        assert!(entry.is_stored(), "id field must be stored");
    }
}
