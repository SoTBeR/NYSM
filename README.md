<div align="center">

<h1>NYSM</h1>

<p><em>Приложение для поиска советских новогодних фильмов по нечёткому запросу</em></p>

![Tauri](https://img.shields.io/badge/Tauri-2-blue?logo=tauri&logoColor=white)
![Svelte](https://img.shields.io/badge/SvelteKit-5-orange?logo=svelte&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue?logo=typescript&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green)

</div>

---

**КиноЗапрос** — десктопное приложение для поиска советских новогодних фильмов с AI-ранжированием результатов. Подборка из 14 тщательно отобранных картин: от классики до забытых жемчужин советского кино.

Введите запрос на естественном языке и получите список фильмов, отсортированных по смысловой близости к вашему запросу. Полнотекстовый поиск учитывает русскую морфологию, AI-слой переранжирует результаты под намерение запроса.

---

## Возможности

- **Полнотекстовый поиск с морфологией** — русский стеммер Snowball обрабатывает все падежи и склонения: «профессия», «профессию», «профессии» найдут одно и то же
- **AI-ранжирование** — модель DeepSeek через [gen-api.ru](https://gen-api.ru) переранжирует найденные фильмы под смысл вашего запроса и поясняет причину выбора каждого
- **Карточки фильмов** — название, год, режиссёр, жанры, длительность, студия; клик открывает полную информацию (описание, актёры, причина ранжирования)
- **Вся коллекция одной кнопкой** — кнопка «Все фильмы» показывает всю базу без поискового запроса
- **Graceful degradation** — при любой ошибке AI результаты всё равно отображаются в порядке Tantivy-релевантности, без сообщений об ошибках
- **Локальные настройки** — API-ключ и base URL хранятся локально через `tauri-plugin-store`, никуда не передаются
- **Советская новогодняя тема** — тёмный фон, красный и золотой, анимированные снежинки, шрифты Playfair Display и IBM Plex Sans

---

## Стек технологий

| Слой | Технология | Версия |
|---|---|---|
| Десктопная оболочка | Tauri | 2 |
| Frontend-фреймворк | SvelteKit + TypeScript | 2.9 / 5.6 |
| UI-библиотека | Svelte (Runes API) | 5 |
| Инструмент сборки | Vite | 6 |
| Полнотекстовый поиск | Tantivy (Rust) | 0.22 |
| База данных | SQLite via `rusqlite` | 0.32 |
| HTTP-клиент | `reqwest` + `tokio` | 0.12 / 1 |
| Сериализация | `serde` / `serde_json` | 1 |
| Персистентность настроек | `tauri-plugin-store` | 2 |
| AI-провайдер | gen-api.ru → `deepseek-chat` | — |

---

## Требования

Перед установкой убедитесь, что у вас есть:

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (стабильный канал)
- Зависимости Tauri v2 для вашей ОС — [инструкция по установке](https://tauri.app/start/prerequisites/)

---

## Быстрый старт

```bash
# Клонировать репозиторий
git clone https://github.com/your-username/nysm.git
cd nysm

# Установить зависимости
npm install

# Запустить в режиме разработки
npm run tauri dev
```

Горячая перезагрузка работает для frontend-части. Изменения в Rust-бэкенде требуют перезапуска.

---

## Настройка AI-ранжирования

По умолчанию приложение работает в режиме чистого полнотекстового поиска. Чтобы включить AI-ранжирование:

1. Зарегистрируйтесь на [gen-api.ru](https://gen-api.ru) и получите API-ключ
2. Откройте настройки приложения — кнопка **⚙** в правом верхнем углу
3. Введите API-ключ (и при необходимости base URL)
4. Переключатель «ИИ-ранжирование» на главной странице станет активным

> Без API-ключа переключатель заблокирован. Приложение полностью работоспособно и без AI — результаты сортируются по BM25-релевантности Tantivy.

---

## Архитектура

### Поток данных

```
Запрос пользователя
  → invoke('search_movies')
      Tantivy: полнотекстовый поиск с русским стеммером
      → Movie[] в порядке BM25-оценки

  → invoke('ai_rank_movies')       ← пропускается если AI выключен
      POST /networks/deepseek-chat → { request_id }
      GET  /request/get/{id}       ← опрос каждые 3 сек, макс. 60 раз
      → RankedMovie[] с объяснениями

  → Рендеринг MovieCard-компонентов
```

При любой ошибке AI (`ai_rank_movies`) возвращается `fallback_ranking` — фильмы в исходном порядке Tantivy с пустым полем `reason`. Пользователь всегда видит результаты.

### Rust-бэкенд (`src-tauri/src/`)

| Файл | Роль |
|---|---|
| `lib.rs` | Точка входа: регистрация плагинов, состояний, команд; хуки запуска |
| `types.rs` | Общие структуры: `Movie`, `RankedMovie`, `AppSettings`, `AppError` |
| `ai.rs` | AI-ранжирование: async POST + опрос, логирование ошибок, fallback |
| `db.rs` | SQLite: `DbState` (Mutex-обёртка), `fetch_all_movies_sync`, команда `get_all_movies_from_db` |
| `search.rs` | Tantivy: русский стеммер-токенайзер, команда `search_movies`, `index_movies_internal` |
| `settings.rs` | `SettingsState` (in-memory кэш) + чтение/запись в `tauri-plugin-store` |

### Frontend (`src/`)

| Путь | Роль |
|---|---|
| `src/routes/+page.svelte` | Единственная страница: форма поиска, AI-переключатель, кнопка «Все фильмы», 5 UI-состояний |
| `src/lib/components/MovieCard.svelte` | Карточка фильма: кликабельная, доступна с клавиатуры (Enter/Space) |
| `src/lib/components/MovieDetailModal.svelte` | Полная информация о фильме: все поля, AI-причина; закрывается по Escape/клику на фон |
| `src/lib/components/SettingsModal.svelte` | Модал настроек: API-ключ и base URL, сохранение через `save_settings` |
| `src/lib/types.ts` | TypeScript-зеркала Rust-структур |
| `src/lib/stores/settings.ts` | Svelte store настроек и флаг `settingsLoaded` |
| `src/app.css` | Дизайн-система: CSS custom properties для цветов, типографики, анимаций |

<details>
<summary>Структура директорий</summary>

```
nysm/
├── src/                              # SvelteKit frontend
│   ├── routes/
│   │   ├── +layout.ts               # ssr = false (Tauri static adapter)
│   │   └── +page.svelte             # Главная страница
│   ├── lib/
│   │   ├── components/
│   │   │   ├── MovieCard.svelte
│   │   │   ├── MovieDetailModal.svelte
│   │   │   └── SettingsModal.svelte
│   │   ├── stores/
│   │   │   └── settings.ts
│   │   └── types.ts
│   └── app.css                      # Дизайн-токены (--red-*, --gold-*, --bg-*)
│
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── ai.rs
│   │   ├── db.rs
│   │   ├── search.rs
│   │   ├── settings.rs
│   │   └── types.rs
│   ├── assets/
│   │   └── movies_database.db       # SQLite-база (14 фильмов, bundled)
│   └── tauri.conf.json
│
├── package.json
└── vite.config.js
```

</details>

### Tauri IPC-команды

| Команда | Rust-обработчик | Параметры / результат |
|---|---|---|
| `search_movies` | `search::search_movies` | `{ query, limit? }` → `Movie[]` |
| `ai_rank_movies` | `ai::ai_rank_movies` | `{ userQuery, movies }` → `RankedMovie[]` |
| `save_settings` | `settings::save_settings` | `{ settings }` → сохранение настроек |
| `load_settings` | `settings::load_settings` | → `AppSettings` |
| `get_all_movies_from_db` | `db::get_all_movies_from_db` | → все 14 фильмов `Movie[]` |

---

## Разработка

```bash
# Только frontend (без Tauri, в браузере)
npm run dev

# Проверка типов (TypeScript + Svelte)
npm run check

# Rust: статический анализ
cd src-tauri && cargo clippy

# Rust: тесты
cd src-tauri && cargo test

# Запуск конкретного теста
cd src-tauri && cargo test fetch_returns_movies
```

---

## Продакшн-сборка

```bash
npm run tauri build
```

Результат — нативный установщик для вашей платформы (`.dmg` на macOS, `.msi`/`.exe` на Windows, `.deb`/`.AppImage` на Linux) в директории `src-tauri/target/release/bundle/`.

---

## База данных

SQLite-файл `src-tauri/assets/movies_database.db` поставляется вместе с приложением через `bundle.resources` в `tauri.conf.json`. Схема: таблицы `movies`, `actors`, `directors`, `genres`, `studios` и таблицы связей (`movie_actors`, `movie_directors`, `movie_genres`, `movie_studios`).

Tantivy-индекс пересоздаётся из SQLite при каждом запуске — при 14 фильмах это занимает миллисекунды и гарантирует актуальность поискового индекса.

---

## Лицензия

MIT
