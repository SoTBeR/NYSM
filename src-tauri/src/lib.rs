mod ai;
mod db;
mod search;
mod settings;
mod types;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(search::SearchState::new())
        .manage(settings::SettingsState::new())
        .manage(db::DbState::new())
        .invoke_handler(tauri::generate_handler![
            // Поиск
            search::search_movies,
            search::index_movies,
            search::rebuild_index,
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

            // 3. Инициализируем Tantivy индекс
            let search_state = app.state::<search::SearchState>();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Cannot resolve app data dir");

            if let Err(e) = search::init_index(&search_state, data_dir) {
                eprintln!("[NYSM] Warning: could not initialise search index: {e}");
            }

            // 4. Заполняем Tantivy из БД (14 фильмов, выполняется быстро)
            //    Пересоздаём индекс при каждом старте — гарантирует актуальность данных.
            let movies = {
                let guard = db_state.0.lock().ok();
                guard
                    .as_ref()
                    .and_then(|g| g.as_ref())
                    .and_then(|conn| db::fetch_all_movies_sync(conn).ok())
            };

            if let Some(movies) = movies {
                match search::index_movies_internal(&search_state, &movies, true) {
                    Ok(()) => eprintln!("[NYSM] Indexed {} movies into Tantivy", movies.len()),
                    Err(e) => eprintln!("[NYSM] Warning: could not index movies: {e}"),
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
