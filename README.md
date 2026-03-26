# NYSM — Новогодний советский кинопоиск

Десктопное приложение для поиска советских новогодних фильмов с AI-ранжированием результатов.

## Возможности

- **Полнотекстовый поиск** по названию, описанию, жанру, режиссёру, актёрам — с учётом русской морфологии (стеммер Snowball)
- **AI-ранжирование** — результаты переранжируются языковой моделью под смысл запроса
- **Карточки фильмов** с кратким описанием; клик открывает полную информацию (актёры, студия, описание, причина ранжирования)
- **Просмотр всей коллекции** одной кнопкой без поискового запроса
- **Настройки** — API-ключ и базовый URL хранятся локально через `tauri-plugin-store`

## Стек

| Слой | Технология |
|---|---|
| Оболочка | Tauri v2 |
| Frontend | SvelteKit 5 + TypeScript + Vite |
| Backend | Rust |
| Полнотекстовый поиск | Tantivy (русский стеммер) |
| База данных | SQLite (`rusqlite`) |
| AI API | gen-api.ru — модель `deepseek-chat` |

## Структура проекта

```
src/                        # SvelteKit frontend
  routes/+page.svelte       # Главная страница
  lib/
    components/
      MovieCard.svelte       # Карточка фильма
      MovieDetailModal.svelte # Модальное окно с полной информацией
      SettingsModal.svelte    # Настройки API
    stores/settings.ts       # Svelte store настроек
    types.ts                 # TypeScript-типы
  app.css                    # Дизайн-система (CSS custom properties)

src-tauri/src/              # Rust backend
  lib.rs                    # Точка входа, регистрация команд
  ai.rs                     # AI-ранжирование (POST/poll)
  db.rs                     # SQLite
  search.rs                 # Tantivy-индекс
  settings.rs               # Настройки
  types.rs                  # Общие структуры

src-tauri/assets/
  movies_database.db        # SQLite-база (14 фильмов)
```

## Запуск

### Требования

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/)

### Разработка

```bash
npm install
npm run tauri dev
```

Горячая перезагрузка работает для frontend; изменения в Rust требуют перезапуска.

### Продакшн-сборка

```bash
npm run tauri build
```

### Только frontend (без Tauri)

```bash
npm run dev
```

### Проверки

```bash
npm run check                        # TypeScript / Svelte
cd src-tauri && cargo clippy         # Rust lints
cd src-tauri && cargo test           # Rust тесты
```

## Настройка AI

1. Откройте настройки (кнопка ⚙ в правом верхнем углу)
2. Введите API-ключ от [gen-api.ru](https://gen-api.ru)
3. Переключатель «ИИ-ранжирование» станет активным

Без API-ключа приложение работает в режиме чистого полнотекстового поиска.

## Дизайн

Советская новогодняя тема — красный и золотой на тёмном фоне. Шрифты: Playfair Display (заголовки) и IBM Plex Sans (текст), оба с поддержкой кириллицы.
