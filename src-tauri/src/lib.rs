mod ai;
mod db;
mod settings;
mod types;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(settings::SettingsState::new())
        .manage(db::DbState::new())
        .invoke_handler(tauri::generate_handler![
            // AI ранжирование
            ai::ai_rank_movies,
            // Настройки
            settings::save_settings,
            settings::load_settings,
            // База данных
            db::get_all_movies_from_db,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. Загружаем настройки из store в in-memory кэш
            let settings_state = app.state::<settings::SettingsState>();
            if let Err(e) = settings::init_settings(&app_handle, &settings_state) {
                eprintln!("[NYSM] Warning: could not load settings: {e}");
            }

            // 2. Открываем SQLite БД
            let db_state = app.state::<db::DbState>();
            let db_path = db::resolve_db_path(&app_handle);
            if let Err(e) = db::init_db(&db_state, db_path) {
                eprintln!("[NYSM] Warning: could not open DB: {e}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
