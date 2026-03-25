use serde::{Deserialize, Serialize};

/// Полная информация о фильме (из SQLite БД)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub actors: Vec<String>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub year: u32,
    pub duration_minutes: Option<u32>,
    pub director: String,
    /// Отсутствует в текущей БД, зарезервировано для будущего
    pub rating: Option<f32>,
}

/// Отранжированный результат после прохода через AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMovie {
    pub movie: Movie,
    /// Позиция в финальном рейтинге (1-based)
    pub rank: u32,
    /// Объяснение от AI, почему фильм подходит (может быть пустым)
    pub reason: String,
}

/// Настройки приложения (сохраняются через tauri-plugin-store)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    /// API ключ нейросетевого агрегатора
    pub ai_api_key: String,
    /// Базовый URL AI API (может быть переопределён пользователем)
    pub ai_base_url: String,
}

/// Унифицированный тип ошибки для команд Tauri
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("AI API error: {0}")]
    Ai(String),
    #[error("Settings error: {0}")]
    Settings(String),
    #[error("Database error: {0}")]
    Database(String),
}

// Tauri требует, чтобы ошибка команды была сериализуемой в JSON
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// --------------------------------------------------------------------------
// Тесты
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_serializes_to_string() {
        let err = AppError::Ai("request failed".into());
        let json = serde_json::to_string(&err).expect("serialize must not fail");
        assert_eq!(json, r#""AI API error: request failed""#);
    }

    #[test]
    fn app_error_settings_variant() {
        let err = AppError::Settings("lock poisoned".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#""Settings error: lock poisoned""#);
    }

    #[test]
    fn app_settings_default_is_empty_strings() {
        let s = AppSettings::default();
        assert_eq!(s.ai_api_key, "");
        assert_eq!(s.ai_base_url, "");
    }

    #[test]
    fn app_settings_roundtrips_through_json() {
        let original = AppSettings {
            ai_api_key: "sk-test-key".into(),
            ai_base_url: "https://api.example.com".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.ai_api_key, original.ai_api_key);
        assert_eq!(restored.ai_base_url, original.ai_base_url);
    }

    #[test]
    fn movie_roundtrips_through_json() {
        let m = Movie {
            id: 42,
            title: "Тест".into(),
            description: "Описание".into(),
            actors: vec!["Актёр 1".into(), "Актёр 2".into()],
            genres: vec!["Комедия".into()],
            studios: vec!["Мосфильм".into()],
            year: 1970,
            duration_minutes: Some(95),
            director: "Режиссёр".into(),
            rating: None,
        };
        let json = serde_json::to_string(&m).unwrap();
        let restored: Movie = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, 42);
        assert_eq!(restored.actors.len(), 2);
        assert_eq!(restored.duration_minutes, Some(95));
    }

    #[test]
    fn ranked_movie_serialization() {
        let rm = RankedMovie {
            movie: Movie {
                id: 1,
                title: "Ирония судьбы".into(),
                description: String::new(),
                actors: vec![],
                genres: vec![],
                studios: vec![],
                year: 1975,
                duration_minutes: Some(184),
                director: "Рязанов".into(),
                rating: None,
            },
            rank: 1,
            reason: "Классика".into(),
        };
        let json = serde_json::to_string(&rm).unwrap();
        assert!(json.contains(r#""rank":1"#));
        assert!(json.contains("Ирония судьбы"));
    }
}
