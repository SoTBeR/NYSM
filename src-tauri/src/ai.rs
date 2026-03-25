//! Интеграция с AI API (gen-api.ru) для ранжирования фильмов.
//!
//! Провайдер работает асинхронно: сначала POST-запрос создаёт задачу
//! и возвращает `request_id`, затем нужно опрашивать GET-эндпоинт
//! пока статус не станет "success", "failed" или другим терминальным.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

use crate::types::{AppError, Movie, RankedMovie};

// --------------------------------------------------------------------------
// Константы
// --------------------------------------------------------------------------

const DEFAULT_API_BASE: &str = "https://api.gen-api.ru";
const MODEL: &str = "claude-3-5-haiku-20241022";
/// Максимум опросов статуса (3 с × 60 = 3 минуты)
const MAX_POLLS: u32 = 60;
const POLL_INTERVAL_SECS: u64 = 3;

// --------------------------------------------------------------------------
// Типы запроса (сериализация в JSON для gen-api.ru)
// --------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum MessageRole {
    System,
    User,
}

#[derive(Serialize)]
struct Message {
    role: MessageRole,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum ReasoningEffort {
    Low,
}

#[derive(Serialize)]
struct AiRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    is_sync: bool,
    stream: bool,
    reasoning_effort: ReasoningEffort,
}

// --------------------------------------------------------------------------
// Типы ответа
// --------------------------------------------------------------------------

/// Ответ на POST-запрос создания задачи.
/// request_id — целое число (u64), возвращаемое gen-api.ru.
#[derive(Deserialize, Debug)]
struct StartResponse {
    request_id: u64,
}

#[derive(Deserialize, Debug)]
struct MessageContent {
    content: String,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize, Debug)]
struct ResultItem {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct StatusResponse {
    status: String,
    result: Option<Vec<ResultItem>>,
}

// --------------------------------------------------------------------------
// Промпты
// --------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = "\
Ты — эксперт по советскому и российскому кино. \
Тебе дан список фильмов с информацией о них и поисковый запрос пользователя. \
Твоя задача: выбрать фильмы, которые подходят под запрос, и отранжировать их по убыванию релевантности.

Верни ТОЛЬКО валидный JSON-массив без какого-либо текста до или после него. \
Ни один фильм не должен встречаться дважды. \
Формат каждого объекта:
{\"movie_id\": <целое число>, \"rank\": <позиция начиная с 1>, \"reason\": \"<1-2 предложения на русском>\"}

Если ни один фильм не подходит, верни пустой массив: []";

/// Формирует текст сообщения пользователя: запрос + список фильмов.
fn build_user_message(query: &str, movies: &[Movie]) -> String {
    let mut msg = format!("Запрос пользователя: {query}\n\nСписок фильмов:\n");
    for m in movies {
        msg.push_str(&format!(
            "\nID: {}\nНазвание: {}\nГод: {}\nДлительность: {} мин\nРежиссёр: {}\nЖанры: {}\nСтудия: {}\nАктёры: {}\nОписание: {}\n---",
            m.id,
            m.title,
            if m.year > 0 { m.year.to_string() } else { "неизвестен".into() },
            m.duration_minutes.map(|d| d.to_string()).unwrap_or_else(|| "—".into()),
            if m.director.is_empty() { "неизвестен" } else { &m.director },
            if m.genres.is_empty() { "не указаны".into() } else { m.genres.join(", ") },
            if m.studios.is_empty() { "не указана".into() } else { m.studios.join(", ") },
            if m.actors.is_empty() { "не указаны".into() } else { m.actors.join(", ") },
            if m.description.is_empty() { "—" } else { &m.description },
        ));
    }
    msg
}

// --------------------------------------------------------------------------
// Парсинг ответа AI
// --------------------------------------------------------------------------

#[derive(Deserialize)]
struct AiRankedItem {
    movie_id: u64,
    rank: u32,
    reason: String,
}

/// Парсит JSON-ответ от AI и сопоставляет `movie_id` с входными фильмами.
/// При ошибке парсинга возвращает фильмы в исходном порядке (fallback).
fn parse_response(raw: &str, movies: &[Movie]) -> Vec<RankedMovie> {
    // AI иногда оборачивает JSON в блок кода ```json ... ```
    let json_str = {
        let trimmed = raw.trim();
        if let Some(inner) = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
        {
            inner.trim_end_matches("```").trim()
        } else {
            trimmed
        }
    };

    let items: Vec<AiRankedItem> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[AI] Failed to parse response JSON: {e}\nRaw: {raw}");
            return fallback_ranking(movies);
        }
    };

    // Строим map id → Movie для быстрого поиска
    let movie_map: std::collections::HashMap<u64, &Movie> =
        movies.iter().map(|m| (m.id, m)).collect();

    let mut ranked: Vec<RankedMovie> = items
        .into_iter()
        .filter_map(|item| {
            movie_map.get(&item.movie_id).map(|&m| RankedMovie {
                movie: m.clone(),
                rank: item.rank,
                reason: item.reason,
            })
        })
        .collect();

    // Сортируем по рангу на случай если AI вернул их не по порядку
    ranked.sort_by_key(|r| r.rank);
    ranked
}

fn fallback_ranking(movies: &[Movie]) -> Vec<RankedMovie> {
    movies
        .iter()
        .enumerate()
        .map(|(i, m)| RankedMovie {
            movie: m.clone(),
            rank: (i + 1) as u32,
            reason: String::new(),
        })
        .collect()
}

// --------------------------------------------------------------------------
// Вспомогательная функция: десериализация с логированием тела при ошибке
// --------------------------------------------------------------------------

async fn read_json<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    context: &str,
) -> Result<T, AppError> {
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Ai(format!("{context}: не удалось прочитать тело ответа: {e}")))?;

    serde_json::from_slice(&bytes).map_err(|e| {
        // Try to pretty-print as JSON to decode \uXXXX escapes into readable Unicode
        let body = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned());
        AppError::Ai(format!("{context}: {e}\nТело ответа:\n{body}"))
    })
}

// --------------------------------------------------------------------------
// Основная функция
// --------------------------------------------------------------------------

/// Отправляет запрос к gen-api.ru и возвращает отранжированные фильмы.
///
/// Поток:
/// 1. POST `/api/v1/networks/claude-4` → `{ request_id }`
/// 2. Цикл GET `/api/v1/request/get/{request_id}` каждые 3 с
/// 3. При статусе "success" — парсим ответ и возвращаем `Vec<RankedMovie>`
pub async fn rank_movies_via_api(
    user_query: &str,
    movies: &[Movie],
    api_key: &str,
    base_url: &str,
) -> Result<Vec<RankedMovie>, AppError> {
    if api_key.is_empty() {
        return Err(AppError::Ai(
            "API ключ не задан. Укажите его в настройках приложения.".into(),
        ));
    }

    let base = if base_url.is_empty() {
        DEFAULT_API_BASE
    } else {
        base_url.trim_end_matches('/')
    };

    let client = Client::new();

    // --- Шаг 1: отправить задачу ---
    let request_body = AiRequest {
        model: MODEL.to_string(),
        messages: vec![
            Message {
                role: MessageRole::System,
                content: SYSTEM_PROMPT.to_string(),
            },
            Message {
                role: MessageRole::User,
                content: build_user_message(user_query, movies),
            },
        ],
        max_tokens: 2000,
        is_sync: false,
        stream: false,
        reasoning_effort: ReasoningEffort::Low,
    };

    let start_url = format!("{base}/api/v1/networks/claude");
    let start_http = client
        .post(&start_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Ai(format!("Ошибка отправки запроса: {e}")))?;

    let start_resp: StartResponse =
        read_json(start_http, "Ошибка разбора ответа запуска").await?;

    let request_id = start_resp.request_id;
    eprintln!("[AI] Request submitted, id={request_id}");

    // --- Шаг 2: опрашиваем статус ---
    let poll_url = format!("{base}/api/v1/request/get/{request_id}");

    for poll in 1..=MAX_POLLS {
        sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let poll_http = client
            .get(&poll_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .await
            .map_err(|e| AppError::Ai(format!("Ошибка опроса статуса: {e}")))?;

        let status_resp: StatusResponse =
            read_json(poll_http, "Ошибка разбора статуса").await?;

        eprintln!("[AI] Poll {poll}/{MAX_POLLS}: status={}", status_resp.status);

        match status_resp.status.as_str() {
            "processing" | "queued" | "pending" => {
                // продолжаем ждать
            }
            "success" => {
                let raw_content = status_resp
                    .result
                    .and_then(|r| r.into_iter().next())
                    .and_then(|item| item.choices.into_iter().next())
                    .map(|c| c.message.content)
                    .unwrap_or_default();

                eprintln!("[AI] Success. Parsing response...");
                return Ok(parse_response(&raw_content, movies));
            }
            "failed" | "error" => {
                return Err(AppError::Ai(format!(
                    "Задача {request_id} завершилась ошибкой на сервере AI"
                )));
            }
            unknown => {
                return Err(AppError::Ai(format!("Неизвестный статус: {unknown}")));
            }
        }
    }

    Err(AppError::Ai(format!(
        "Превышено время ожидания ответа AI (задача {request_id})"
    )))
}

// --------------------------------------------------------------------------
// Tauri команда
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_rank_movies(
    user_query: String,
    movies: Vec<Movie>,
    settings_state: tauri::State<'_, crate::settings::SettingsState>,
) -> Result<Vec<RankedMovie>, AppError> {
    let settings = settings_state.load()?;
    rank_movies_via_api(&user_query, &movies, &settings.ai_api_key, &settings.ai_base_url).await
}
