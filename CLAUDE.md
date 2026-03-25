# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Run the app (dev mode)
```bash
npm run tauri dev
```
This runs `npm run dev` (Vite) and the Tauri shell concurrently. Hot-reload works for the frontend; Rust changes require a restart.

### Frontend only (no Tauri shell)
```bash
npm run dev
```

### Type-check frontend
```bash
npm run check
```

### Rust checks
```bash
cd src-tauri && cargo check
cd src-tauri && cargo clippy
cd src-tauri && cargo test
```

### Run a single Rust test
```bash
cd src-tauri && cargo test <test_name>
# e.g. cargo test search_cyrillic_returns_results
```

### Build for production
```bash
npm run tauri build
```

## Architecture

### Data flow
```
User query
  → invoke('search_movies')      — Tantivy full-text search (local index)
  → invoke('ai_rank_movies')     — AI API re-ranking
  → RankedMovie[]                — rendered as MovieCard components
```

On app startup (`lib.rs` `.setup()`):
1. Settings are read from `tauri-plugin-store` (`nysm_settings.json`) into `SettingsState` in-memory cache.
2. Tantivy index is opened/created in `{app_data_dir}/nysm_index/`.

### Rust backend (`src-tauri/src/`)

| File | Role |
|---|---|
| `lib.rs` | App entry: plugin registration, state management, command handler registration, startup hooks |
| `types.rs` | Shared structs: `Movie`, `SearchResult`, `RankedMovie`, `AppSettings`, `AppError` |
| `search.rs` | Tantivy integration: `SearchState` (Mutex-wrapped index), `fuzzy_search()`, `init_index()`, Tauri commands |
| `ai.rs` | **STUB** — AI ranking placeholder. Replace `rank_movies()` when API code arrives |
| `db.rs` | **STUB** — `MovieRepository` trait + `MockMovieRepository` (5 sample films). `PostgresMovieRepository` is a skeleton awaiting credentials |
| `settings.rs` | `SettingsState` in-memory cache + `tauri-plugin-store` read/write for `AppSettings` |

### Frontend (`src/`)

| Path | Role |
|---|---|
| `src/routes/+page.svelte` | Single page: search form, 5 UI states (idle/searching/ranking/done/error) |
| `src/lib/components/MovieCard.svelte` | Film card: poster, title, year, director, genres, rating, AI reason |
| `src/lib/components/SettingsModal.svelte` | Overlay modal: API key + base URL fields, persists via `save_settings` |
| `src/lib/types.ts` | TypeScript mirrors of Rust structs (`Movie`, `RankedMovie`, `AppSettings`, etc.) |
| `src/lib/stores/settings.ts` | Svelte stores for `settingsStore` and `settingsLoaded` flag |
| `src/app.css` | Design system: CSS custom properties for all colors, spacing, typography, animations |
| `src/routes/+layout.ts` | Sets `ssr = false` (required for Tauri's static adapter) |

### Tauri IPC command names
All commands use snake_case. Tauri auto-converts camelCase JS object keys to snake_case Rust params.

| Command | Rust handler | Notes |
|---|---|---|
| `search_movies` | `search::search_movies` | `{ query, limit? }` → `SearchResult[]` |
| `ai_rank_movies` | `ai::ai_rank_movies` | `{ userQuery, movies }` → `RankedMovie[]` |
| `save_settings` | `settings::save_settings` | `{ settings }` → writes store + updates cache |
| `load_settings` | `settings::load_settings` | reads store, updates cache → `AppSettings` |
| `get_all_movies_from_db` | `db::get_all_movies_from_db` | currently uses `MockMovieRepository` |
| `index_movies` | `search::index_movies` | `{ movies, clearFirst? }` → indexes into Tantivy |
| `rebuild_index` | `search::rebuild_index` | clears index, re-indexes all movies |

### Design system
Defined entirely in `src/app.css` via CSS custom properties. Key variable families:
- `--red-*`, `--gold-*` — primary palette (Soviet New Year theme)
- `--bg-*` — background layers
- `--text-*`, `--border-*` — text and border tokens
- `--font-display` (Playfair Display), `--font-body` (IBM Plex Sans) — both support Cyrillic
- `--transition-base`, `--radius-*`, `--shadow-*`, `--space-*` — motion and layout tokens

## Key Implementation Notes

### Cyrillic search
`FuzzyTermQuery` is intentionally absent — it computes Levenshtein distance over raw UTF-8 bytes, making it unable to match Cyrillic characters (2 bytes each). `QueryParser` is used instead, which tokenizes and matches Cyrillic correctly. "Fuzzy" ranking is delegated to the AI layer.

### Tantivy reader reload
`idx.reader.reload()` is called explicitly after every `writer.commit()`. Without this, `OnCommitWithDelay` polling causes a race condition where newly indexed documents are invisible to the searcher for up to 500 ms.

### Settings flow
`SettingsState` is an in-memory Mutex cache populated at startup and on every `load_settings`/`save_settings` call. The AI module reads settings via `settings_state.load()` — it does not access the store directly.

## Pending

- **API key**: user enters it via Settings UI → saved in `nysm_settings.json` via `tauri-plugin-store`. Nothing to code.

## Database (SQLite)

- File: `src-tauri/assets/movies_database.db` — bundled into the app via `tauri.conf.json` → `bundle.resources`
- Schema: `movies`, `actors`, `directors`, `genres`, `studios` + junction tables `movie_actors`, `movie_directors`, `movie_genres`, `movie_studios`
- Key columns: `movie_id`, `title`, `description`, `release_year`, `duration_minutes`; related data via GROUP_CONCAT JOINs
- Dev path: `$CARGO_MANIFEST_DIR/assets/movies_database.db` (resolved in `db::resolve_db_path`)
- Release path: `$RESOURCE_DIR/movies_database.db`

## AI API (gen-api.ru)

- Endpoint: `POST https://api.gen-api.ru/api/v1/networks/claude-4` → `{request_id}`
- Polling: `GET https://api.gen-api.ru/api/v1/request/get/{request_id}` every 3s, max 60 polls
- Model: `claude-opus-4-5`, `reasoning_effort: low`, `max_tokens: 2000`
- Response: JSON array `[{movie_id, rank, reason}]` — parsed in `parse_response()` with fallback to original order on parse error
