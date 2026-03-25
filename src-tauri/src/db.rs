//! Модуль доступа к SQLite базе данных фильмов.
//!
//! БД bundled внутри приложения: `src-tauri/assets/movies_database.db`.
//! В production читается из `$RESOURCE_DIR/movies_database.db`.
//! В режиме разработки — из `$CARGO_MANIFEST_DIR/assets/movies_database.db`.
//!
//! Схема (упрощённо):
//!   movies(movie_id, title, description, release_year, duration_minutes)
//!   actors, directors, genres, studios — справочники
//!   movie_actors, movie_directors, movie_genres, movie_studios — связи M:N

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

use crate::types::{AppError, Movie};

// --------------------------------------------------------------------------
// State
// --------------------------------------------------------------------------

pub struct DbState(pub Mutex<Option<Connection>>);

impl DbState {
    pub fn new() -> Self {
        DbState(Mutex::new(None))
    }
}

// --------------------------------------------------------------------------
// Инициализация
// --------------------------------------------------------------------------

/// Открывает соединение с БД по пути `db_path`.
/// Включает foreign keys и WAL-режим для безопасного параллельного чтения.
pub fn init_db(state: &DbState, db_path: PathBuf) -> Result<(), AppError> {
    if !db_path.exists() {
        return Err(AppError::Database(format!(
            "DB file not found: {}",
            db_path.display()
        )));
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| AppError::Database(format!("Cannot open DB at {}: {e}", db_path.display())))?;

    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )
    .map_err(|e| AppError::Database(format!("PRAGMA error: {e}")))?;

    let mut guard = state
        .0
        .lock()
        .map_err(|_| AppError::Database("Lock poisoned".into()))?;
    *guard = Some(conn);
    Ok(())
}

/// Возвращает путь к файлу БД.
/// В dev-сборке (`debug_assertions`) — из директории cargo-проекта.
/// В release — из bundled ресурсов Tauri.
pub fn resolve_db_path(app: &tauri::AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("movies_database.db")
    } else {
        app.path()
            .resource_dir()
            .expect("Cannot resolve resource dir")
            .join("movies_database.db")
    }
}

// --------------------------------------------------------------------------
// Вспомогательная функция
// --------------------------------------------------------------------------

/// Разбивает строку `"a, b, c"` (результат GROUP_CONCAT) в `Vec<String>`.
fn csv_to_vec(s: Option<String>) -> Vec<String> {
    s.unwrap_or_default()
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

// --------------------------------------------------------------------------
// Основной запрос
// --------------------------------------------------------------------------

/// Загружает все фильмы из БД одним запросом с GROUP_CONCAT для связанных данных.
///
/// Использует GROUP BY + GROUP_CONCAT чтобы избежать дублирования строк при JOIN.
pub fn fetch_all_movies_sync(conn: &Connection) -> Result<Vec<Movie>, AppError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                m.movie_id,
                m.title,
                COALESCE(m.description, '')            AS description,
                m.release_year,
                m.duration_minutes,
                GROUP_CONCAT(DISTINCT TRIM(d.director_name)) AS directors,
                GROUP_CONCAT(DISTINCT TRIM(a.actor_name))    AS actors,
                GROUP_CONCAT(DISTINCT TRIM(g.genre_name))    AS genres,
                GROUP_CONCAT(DISTINCT TRIM(s.studio_name))   AS studios
            FROM movies m
            LEFT JOIN movie_directors md ON m.movie_id = md.movie_id
            LEFT JOIN directors d        ON md.director_id = d.director_id
            LEFT JOIN movie_actors   ma  ON m.movie_id = ma.movie_id
            LEFT JOIN actors a           ON ma.actor_id = a.actor_id
            LEFT JOIN movie_genres   mg  ON m.movie_id = mg.movie_id
            LEFT JOIN genres g           ON mg.genre_id = g.genre_id
            LEFT JOIN movie_studios  ms  ON m.movie_id = ms.movie_id
            LEFT JOIN studios s          ON ms.studio_id = s.studio_id
            GROUP BY m.movie_id
            ORDER BY m.release_year ASC
            "#,
        )
        .map_err(|e| AppError::Database(format!("Prepare error: {e}")))?;

    let movies = stmt
        .query_map([], |row| {
            let directors_csv: Option<String> = row.get(5)?;
            let actors_csv: Option<String> = row.get(6)?;
            let genres_csv: Option<String> = row.get(7)?;
            let studios_csv: Option<String> = row.get(8)?;

            // Берём первого режиссёра как основного
            let directors = csv_to_vec(directors_csv);
            let director = directors.first().cloned().unwrap_or_default();

            Ok(Movie {
                id: row.get::<_, i64>(0)? as u64,
                title: row.get(1)?,
                description: row.get(2)?,
                year: row
                    .get::<_, Option<i32>>(3)?
                    .unwrap_or(0)
                    .max(0) as u32,
                duration_minutes: row
                    .get::<_, Option<i32>>(4)?
                    .map(|v| v.max(0) as u32),
                director,
                actors: csv_to_vec(actors_csv),
                genres: csv_to_vec(genres_csv),
                studios: csv_to_vec(studios_csv),
                rating: None,
            })
        })
        .map_err(|e| AppError::Database(format!("Query error: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Database(format!("Row error: {e}")))?;

    Ok(movies)
}

// --------------------------------------------------------------------------
// Tauri команды
// --------------------------------------------------------------------------

/// Возвращает все фильмы из БД.
/// Вызывается фронтендом для первичной загрузки и AI-ранжирования.
#[tauri::command]
pub async fn get_all_movies_from_db(
    state: tauri::State<'_, DbState>,
) -> Result<Vec<Movie>, AppError> {
    // Держим lock только пока выполняем синхронный запрос, до первого .await
    let movies = {
        let guard = state
            .0
            .lock()
            .map_err(|_| AppError::Database("Lock poisoned".into()))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| AppError::Database("DB not initialised".into()))?;
        fetch_all_movies_sync(conn)?
    };
    Ok(movies)
}

// --------------------------------------------------------------------------
// Тесты
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("movies_database.db")
    }

    fn open_test_conn() -> Connection {
        let path = test_db_path();
        assert!(path.exists(), "Test DB not found at {}", path.display());
        let conn = Connection::open(&path).expect("open DB");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn db_file_exists() {
        assert!(test_db_path().exists());
    }

    #[test]
    fn fetch_returns_movies() {
        let conn = open_test_conn();
        let movies = fetch_all_movies_sync(&conn).expect("fetch must succeed");
        assert!(!movies.is_empty(), "DB must contain at least one movie");
    }

    #[test]
    fn all_movies_have_title() {
        let conn = open_test_conn();
        let movies = fetch_all_movies_sync(&conn).unwrap();
        for m in &movies {
            assert!(!m.title.is_empty(), "movie id={} has empty title", m.id);
        }
    }

    #[test]
    fn all_movies_have_nonzero_id() {
        let conn = open_test_conn();
        let movies = fetch_all_movies_sync(&conn).unwrap();
        for m in &movies {
            assert!(m.id > 0, "movie '{}' has id=0", m.title);
        }
    }

    #[test]
    fn movies_have_actors_and_genres() {
        let conn = open_test_conn();
        let movies = fetch_all_movies_sync(&conn).unwrap();
        let with_actors = movies.iter().filter(|m| !m.actors.is_empty()).count();
        let with_genres = movies.iter().filter(|m| !m.genres.is_empty()).count();
        assert!(with_actors > 0, "no movies have actors");
        assert!(with_genres > 0, "no movies have genres");
    }

    #[test]
    fn csv_to_vec_splits_correctly() {
        let v = csv_to_vec(Some("Андрей Мягков, Барбара Брыльска".to_string()));
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], "Андрей Мягков");
        assert_eq!(v[1], "Барбара Брыльска");
    }

    #[test]
    fn csv_to_vec_handles_none() {
        assert!(csv_to_vec(None).is_empty());
    }
}
