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
const MODEL: &str = "deepseek-chat";
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
struct ResultItem {
    message: MessageContent,
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
Тебе дан список фильмов с их ID и информацией о них, а также поисковый запрос пользователя. \
Твоя задача — отобрать только действительно подходящие фильмы и упорядочить их по убыванию релевантности.\n\
Правила отбора:\n\
- Включай фильм ТОЛЬКО если есть явная, конкретная связь с запросом: по теме, жанру, персонажам, сюжету, названию, режиссёру или актёрам.\n\
- Косвенной или надуманной связи недостаточно — если нужно долго объяснять, почему фильм подходит, он не подходит.\n\
- Исключай фильмы, которые совпадают с запросом лишь по общим словам (например, «кино», «фильм», «советский»).\n\
- Лучше вернуть меньше фильмов, но все — по делу.\n\
Верни ТОЛЬКО валидный JSON-массив целых чисел — ID подходящих фильмов в порядке убывания релевантности. \
Без какого-либо текста до или после массива. \
Пример: [3,1,7,2]";

/// Формирует текст сообщения пользователя: запрос + только совпавшие поля каждого фильма.
///
/// Всегда включаются ID, название и год. Остальные поля включаются только если
/// содержат хотя бы один из токенов запроса — это сокращает размер промпта.
fn build_user_message(query: &str, movies: &[Movie]) -> String {
    // Токенизируем запрос: разбиваем по пробелам и ASCII-пунктуации, минимальная длина 2
    let terms: Vec<String> = query
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect();

    let matches = |text: &str| -> bool {
        let text_lower = text.to_lowercase();
        terms.iter().any(|t| {
            // Exact substring match first
            if text_lower.contains(t.as_str()) {
                return true;
            }
            // Prefix match: compare first min(6, len) chars of term against each word in text.
            // Handles Russian inflections: профессия/профессию/профессии all share prefix "профес"
            let t_prefix = &t[..t.char_indices().nth(6).map(|(i, _)| i).unwrap_or(t.len())];
            if t_prefix.chars().count() >= 4 {
                text_lower.split_whitespace().any(|word| {
                    let w_prefix =
                        &word[..word.char_indices().nth(6).map(|(i, _)| i).unwrap_or(word.len())];
                    t_prefix == w_prefix
                })
            } else {
                false
            }
        })
    };

    let mut msg = format!("Запрос пользователя: {query}\n\nФильмы (ID и совпавшие поля):\n");

    for m in movies {
        let mut parts: Vec<String> = Vec::new();

        // ID и название — всегда (основной идентификатор)
        parts.push(format!("ID:{}", m.id));
        parts.push(format!("Название:{}", m.title));

        // Год — всегда (короткий, полезный контекст)
        if m.year > 0 {
            parts.push(format!("Год:{}", m.year));
        }

        // Режиссёр — только если совпадает
        if !m.director.is_empty() && matches(&m.director) {
            parts.push(format!("Режиссёр:{}", m.director));
        }

        // Актёры — только совпавшие
        let matched_actors: Vec<&str> = m
            .actors
            .iter()
            .filter(|a| matches(a))
            .map(|a| a.as_str())
            .collect();
        if !matched_actors.is_empty() {
            parts.push(format!("Актёры:{}", matched_actors.join(", ")));
        }

        // Жанры — только совпавшие
        let matched_genres: Vec<&str> = m
            .genres
            .iter()
            .filter(|g| matches(g))
            .map(|g| g.as_str())
            .collect();
        if !matched_genres.is_empty() {
            parts.push(format!("Жанры:{}", matched_genres.join(", ")));
        }

        // Описание — только если совпадает, усечённое до 120 символов (кодовых точек)
        if !m.description.is_empty() && matches(&m.description) {
            let desc = if m.description.chars().count() > 120 {
                let byte_end = m
                    .description
                    .char_indices()
                    .nth(120)
                    .map(|(i, _)| i)
                    .unwrap_or(m.description.len());
                format!("{}…", &m.description[..byte_end])
            } else {
                m.description.clone()
            };
            parts.push(format!("Описание:{desc}"));
        }

        msg.push_str(&parts.join(" | "));
        msg.push('\n');
    }

    msg
}

// --------------------------------------------------------------------------
// Парсинг ответа AI
// --------------------------------------------------------------------------

/// Парсит JSON-ответ от AI (массив ID) и сопоставляет с входными фильмами.
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

    let ids: Vec<u64> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[AI] Failed to parse response JSON: {e}\nRaw: {raw}");
            return fallback_ranking(movies);
        }
    };

    let movie_map: std::collections::HashMap<u64, &Movie> =
        movies.iter().map(|m| (m.id, m)).collect();

    ids.into_iter()
        .enumerate()
        .filter_map(|(i, id)| {
            movie_map.get(&id).map(|&m| RankedMovie {
                movie: m.clone(),
                rank: (i + 1) as u32,
                reason: String::new(),
            })
        })
        .collect()
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
/// 1. POST `/api/v1/networks/` → `{ request_id }`
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

    let start_url = format!("{base}/api/v1/networks/deepseek-chat");
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
                    .map(|item| item.message.content)
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
    match rank_movies_via_api(&user_query, &movies, &settings.ai_api_key, &settings.ai_base_url)
        .await
    {
        Ok(ranked) => Ok(ranked),
        Err(e) => {
            eprintln!("[AI] API unavailable, falling back to local ranking: {e}");
            Ok(fallback_ranking(&movies))
        }
    }
}
