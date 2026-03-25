//! AI API интеграция для ранжирования фильмов
//!
//! СТАТУС: заглушка (placeholder).
//! Реальный код для нейросетевого агрегатора будет добавлен позже,
//! когда будут предоставлены API ключ и документация.
//!
//! Текущее поведение:
//! - Принимает запрос пользователя + список фильмов из Tantivy
//! - Возвращает те же фильмы с рангами по порядку (без реального AI ранжирования)
//! - Логирует что был вызван stub

use crate::types::{AppError, Movie, RankedMovie};

/// Запрос к AI API
#[derive(Debug)]
pub struct AiRankRequest {
    pub user_query: String,
    pub movies: Vec<Movie>,
}

/// Ответ AI API (внутренний формат перед маппингом в RankedMovie)
#[derive(Debug)]
pub struct AiRankResponse {
    pub ranked: Vec<RankedMovie>,
}

// --------------------------------------------------------------------------
// AI клиент (placeholder — будет заменён на реальную реализацию)
// --------------------------------------------------------------------------

/// Отправляет запрос к AI API и возвращает отранжированные фильмы.
///
/// # Placeholder behaviour
/// Пока реальный API не подключён, функция возвращает фильмы в исходном порядке
/// (как пришли из Tantivy), присваивая им последовательные ранги.
///
/// # TODO: реальная реализация
/// 1. Сформировать промпт с описаниями фильмов и запросом пользователя
/// 2. POST на `{base_url}/chat/completions` (или аналог агрегатора)
/// 3. Разобрать ответ: ожидаем JSON-массив с id фильмов в порядке релевантности
/// 4. Сопоставить id с входными фильмами и вернуть RankedMovie
pub async fn rank_movies(
    request: AiRankRequest,
    _api_key: &str,
    _base_url: &str,
) -> Result<AiRankResponse, AppError> {
    // --- STUB START ---
    // Просто возвращаем фильмы в том порядке, в котором они пришли
    eprintln!(
        "[AI STUB] rank_movies called: query='{}', {} films",
        request.user_query,
        request.movies.len()
    );

    let ranked = request
        .movies
        .into_iter()
        .enumerate()
        .map(|(i, movie)| RankedMovie {
            movie,
            rank: (i + 1) as u32,
            reason: String::new(), // AI объяснение будет здесь
        })
        .collect();

    Ok(AiRankResponse { ranked })
    // --- STUB END ---
}

// --------------------------------------------------------------------------
// Tauri команды
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_rank_movies(
    user_query: String,
    movies: Vec<Movie>,
    settings_state: tauri::State<'_, crate::settings::SettingsState>,
) -> Result<Vec<RankedMovie>, AppError> {
    let settings = settings_state.load()?;

    let request = AiRankRequest { user_query, movies };

    let response = rank_movies(request, &settings.ai_api_key, &settings.ai_base_url).await?;

    Ok(response.ranked)
}
