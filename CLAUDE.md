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
# e.g. cargo test fetch_returns_movies
```

### Build for production
```bash
npm run tauri build
```

## Architecture

### Data flow
```
User query
  → invoke('search_movies')     — Tantivy full-text search → Vec<Movie> (pre-filtered, score order)
  → invoke('ai_rank_movies')    — AI API re-ranking (skipped if user toggled AI off)
  → RankedMovie[]               — rendered as MovieCard components
```

If AI is disabled via the toggle, `search_movies` results are converted directly to `RankedMovie[]` preserving Tantivy score order (empty `reason`).

On app startup (`lib.rs` `.setup()`):
1. Settings are read from `tauri-plugin-store` (`nysm_settings.json`) into `SettingsState` in-memory cache.
2. SQLite DB is opened into `DbState`.
3. Tantivy index is initialised from `app_data_dir`.
4. All 14 movies are fetched from SQLite and indexed into Tantivy (`clear_first: true`).

### Rust backend (`src-tauri/src/`)

| File | Role |
|---|---|
| `lib.rs` | App entry: plugin registration, state management, command handler registration, startup hooks |
| `types.rs` | Shared structs: `Movie`, `RankedMovie`, `AppSettings`, `AppError` |
| `ai.rs` | AI ranking via gen-api.ru — async POST/poll flow, raw body logging on parse errors, fallback to local order on API failure |
| `db.rs` | SQLite integration: `DbState` (Mutex-wrapped connection), `fetch_all_movies_sync()`, `get_all_movies_from_db` Tauri command |
| `search.rs` | Tantivy full-text index: Russian stemmer tokenizer, `search_movies` Tauri command, `index_movies_internal` |
| `settings.rs` | `SettingsState` in-memory cache + `tauri-plugin-store` read/write for `AppSettings` |

### Frontend (`src/`)

| Path | Role |
|---|---|
| `src/routes/+page.svelte` | Single page: search form, AI toggle, 5 UI states (idle/searching/ranking/done/error) |
| `src/lib/components/MovieCard.svelte` | Film card: title, year, director, genres, rating, AI reason (hidden when empty) |
| `src/lib/components/SettingsModal.svelte` | Overlay modal: API key + base URL fields, persists via `save_settings` |
| `src/lib/types.ts` | TypeScript mirrors of Rust structs (`Movie`, `RankedMovie`, `AppSettings`, etc.) |
| `src/lib/stores/settings.ts` | Svelte stores for `settingsStore` and `settingsLoaded` flag |
| `src/app.css` | Design system: CSS custom properties for all colors, spacing, typography, animations |
| `src/routes/+layout.ts` | Sets `ssr = false` (required for Tauri's static adapter) |

### Tauri IPC command names
All commands use snake_case. Tauri auto-converts camelCase JS object keys to snake_case Rust params.

| Command | Rust handler | Notes |
|---|---|---|
| `search_movies` | `search::search_movies` | `{ query, limit? }` → Tantivy search → `Movie[]` in score order |
| `ai_rank_movies` | `ai::ai_rank_movies` | `{ userQuery, movies }` → `RankedMovie[]`; falls back to input order on API failure |
| `save_settings` | `settings::save_settings` | `{ settings }` → writes store + updates cache |
| `load_settings` | `settings::load_settings` | reads store, updates cache → `AppSettings` |
| `get_all_movies_from_db` | `db::get_all_movies_from_db` | returns all 14 movies from SQLite → `Movie[]` |

### Design system
Defined entirely in `src/app.css` via CSS custom properties. Key variable families:
- `--red-*`, `--gold-*` — primary palette (Soviet New Year theme)
- `--bg-*` — background layers
- `--text-*`, `--border-*` — text and border tokens
- `--font-display` (Playfair Display), `--font-body` (IBM Plex Sans) — both support Cyrillic
- `--transition-base`, `--radius-*`, `--shadow-*`, `--space-*` — motion and layout tokens

## Key Implementation Notes

### Search approach
Tantivy is used as a pre-filter: `search_movies` queries the local index (Russian Snowball stemmer — handles inflected forms like профессия/профессию) and returns matching `Movie[]` in score order. The AI layer then re-ranks this subset. With only 14 films the index is rebuilt from SQLite on every startup.

### AI context — compact field selection
`build_user_message` in `ai.rs` only includes fields that match query terms per movie (exact substring + 6-char prefix heuristic for Russian inflections). `id`, `title`, and `year` are always included. This keeps AI prompts short.

### AI fallback
If `ai_rank_movies` fails for any reason (network, parse error, etc.), it silently returns `fallback_ranking` — movies in their input (Tantivy score) order with empty reasons. The error is logged to stderr. The frontend always gets results.

### Settings flow
`SettingsState` is an in-memory Mutex cache populated at startup and on every `load_settings`/`save_settings` call. The AI module reads settings via `settings_state.load()` — it does not access the store directly.

### AI API error debugging
`ai.rs` reads raw response bytes before JSON deserialization. On parse failure it pretty-prints the JSON (decoding `\uXXXX` escapes) and logs both the serde error and full body to stderr.

## Pending

- **API key**: user enters it via Settings UI → saved in `nysm_settings.json` via `tauri-plugin-store`. Nothing to code.

## Database (SQLite)

- File: `src-tauri/assets/movies_database.db` — bundled into the app via `tauri.conf.json` → `bundle.resources`
- Schema: `movies`, `actors`, `directors`, `genres`, `studios` + junction tables `movie_actors`, `movie_directors`, `movie_genres`, `movie_studios`
- Key columns: `movie_id`, `title`, `description`, `release_year`, `duration_minutes`; related data via GROUP_CONCAT JOINs
- Dev path: `$CARGO_MANIFEST_DIR/assets/movies_database.db` (resolved in `db::resolve_db_path`)
- Release path: `$RESOURCE_DIR/movies_database.db`

## AI API (gen-api.ru)

- Endpoint: `POST https://api.gen-api.ru/api/v1/networks/claude-4` → `{ request_id: u64 }`
- Polling: `GET https://api.gen-api.ru/api/v1/request/get/{request_id}` every 3s, max 60 polls
- Model: configured via `MODEL` constant in `ai.rs`, `reasoning_effort: low`, `max_tokens: 2000`
- Response: AI returns a JSON array of integer movie IDs in relevance order — parsed in `parse_response()`, fallback to input order on parse error
- `request_id` is a **u64 integer** — the `StartResponse` struct uses `u64`
- Status response shape: `{ status, result: [{ message: { content } }] }` (no `choices` wrapper)
