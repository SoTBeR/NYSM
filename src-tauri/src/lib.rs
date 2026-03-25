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
        .manage(settings::SettingsState::new())
        .manage(db::DbState::new())
        .manage(search::SearchState::new())
        .invoke_handler(tauri::generate_handler![
            // AI ранжирование
            ai::ai_rank_movies,
            // Настройки
            settings::save_settings,
            settings::load_settings,
            // База данных
            db::get_all_movies_from_db,
            // Поиск (Tantivy)
            search::search_movies,
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

            // 3. Инициализируем Tantivy-индекс в app_data_dir
            let search_state = app.state::<search::SearchState>();
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Cannot resolve app_data_dir");

            if let Err(e) = search::init_index(&search_state, app_data_dir) {
                eprintln!("[NYSM] Warning: could not init Tantivy index: {e}");
            }

            // 4. Загружаем все фильмы из БД и индексируем в Tantivy
            {
                let movies_result = {
                    let guard = db_state
                        .0
                        .lock()
                        .expect("DB lock must not be poisoned at startup");
                    guard
                        .as_ref()
                        .map(|conn| db::fetch_all_movies_sync(conn))
                };

                match movies_result {
                    Some(Ok(movies)) => {
                        eprintln!("[NYSM] Indexing {} movies into Tantivy...", movies.len());
                        if let Err(e) =
                            search::index_movies_internal(&search_state, &movies, true)
                        {
                            eprintln!("[NYSM] Warning: could not index movies: {e}");
                        } else {
                            eprintln!("[NYSM] Tantivy index ready.");
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("[NYSM] Warning: could not fetch movies for indexing: {e}");
                    }
                    None => {
                        eprintln!("[NYSM] Warning: DB not available, skipping initial indexing.");
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
