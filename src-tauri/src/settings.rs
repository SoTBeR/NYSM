use std::sync::Mutex;

use tauri_plugin_store::StoreExt;

use crate::types::{AppError, AppSettings};

/// Ключ в хранилище tauri-plugin-store
const STORE_KEY: &str = "settings";

/// Имя файла хранилища (создаётся в app data dir)
const STORE_FILE: &str = "nysm_settings.json";

// --------------------------------------------------------------------------
// In-memory кэш настроек (чтобы не читать с диска при каждом вызове)
// --------------------------------------------------------------------------

pub struct SettingsState(pub Mutex<AppSettings>);

impl SettingsState {
    pub fn new() -> Self {
        SettingsState(Mutex::new(AppSettings::default()))
    }

    /// Возвращает текущие настройки из in-memory кэша
    pub fn load(&self) -> Result<AppSettings, AppError> {
        let guard = self
            .0
            .lock()
            .map_err(|_| AppError::Settings("Lock poisoned".into()))?;
        Ok(guard.clone())
    }

    /// Обновляет in-memory кэш
    pub fn update(&self, settings: AppSettings) -> Result<(), AppError> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| AppError::Settings("Lock poisoned".into()))?;
        *guard = settings;
        Ok(())
    }
}

// --------------------------------------------------------------------------
// Вспомогательные функции для чтения/записи через tauri-plugin-store
// --------------------------------------------------------------------------

fn read_from_store(app: &tauri::AppHandle) -> Result<AppSettings, AppError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| AppError::Settings(format!("Cannot open store: {e}")))?;

    let settings = match store.get(STORE_KEY) {
        Some(value) => serde_json::from_value::<AppSettings>(value.clone())
            .map_err(|e| AppError::Settings(format!("Deserialization error: {e}")))?,
        None => AppSettings::default(),
    };

    Ok(settings)
}

fn write_to_store(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), AppError> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| AppError::Settings(format!("Cannot open store: {e}")))?;

    let value = serde_json::to_value(settings)
        .map_err(|e| AppError::Settings(format!("Serialization error: {e}")))?;

    store.set(STORE_KEY, value);

    store
        .save()
        .map_err(|e| AppError::Settings(format!("Save error: {e}")))?;

    Ok(())
}

// --------------------------------------------------------------------------
// Tauri команды
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    app: tauri::AppHandle,
    state: tauri::State<'_, SettingsState>,
) -> Result<(), AppError> {
    write_to_store(&app, &settings)?;
    state.update(settings)?;
    Ok(())
}

#[tauri::command]
pub async fn load_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, SettingsState>,
) -> Result<AppSettings, AppError> {
    let settings = read_from_store(&app)?;
    state.update(settings.clone())?;
    Ok(settings)
}

/// Инициализация настроек при старте приложения -
/// читает из store и заполняет in-memory кэш.
pub fn init_settings(app: &tauri::AppHandle, state: &SettingsState) -> Result<(), AppError> {
    let settings = read_from_store(app).unwrap_or_default();
    state.update(settings)
}
