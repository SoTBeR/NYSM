<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settingsStore, settingsLoaded } from '$lib/stores/settings';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import MovieCard from '$lib/components/MovieCard.svelte';
  import MovieDetailModal from '$lib/components/MovieDetailModal.svelte';
  import type { Movie, RankedMovie, AppSettings } from '$lib/types';

  // --- State ---
  let query = $state('');
  let searchState: 'idle' | 'searching' | 'ranking' | 'done' | 'error' = $state('idle');
  let results = $state<RankedMovie[]>([]);
  let errorMsg = $state('');
  let settingsOpen = $state(false);
  let useAi = $state(true);
  let selectedMovie = $state<RankedMovie | null>(null);
  let loadingAll = $state(false);

  // Search input ref
  let inputEl: HTMLInputElement | null = $state(null);

  // --- Init: load settings ---
  onMount(async () => {
    try {
      const settings = await invoke<AppSettings>('load_settings');
      settingsStore.set(settings);
      settingsLoaded.set(true);
    } catch {
      // Настройки не критичны при запуске
      settingsLoaded.set(true);
    }
  });

  // --- Search handler ---
  async function handleSearch(e?: Event) {
    e?.preventDefault();
    const q = query.trim();
    if (!q) return;

    searchState = 'searching';
    errorMsg = '';
    results = [];

    try {
      // Шаг 1: полнотекстовый поиск через Tantivy (pre-filter)
      const movies = await invoke<Movie[]>('search_movies', { query: q, limit: 20 });

      if (movies.length === 0) {
        searchState = 'done';
        results = [];
        return;
      }

      if (useAi) {
        // Шаг 2: AI ранжирование совпавших фильмов
        searchState = 'ranking';
        const ranked = await invoke<RankedMovie[]>('ai_rank_movies', {
          userQuery: q,
          movies,
        });
        results = ranked;
      } else {
        // Локальный режим: конвертируем Movie[] → RankedMovie[], сохраняя порядок Tantivy
        results = movies.map((movie, i) => ({ movie, rank: i + 1, reason: '' }));
      }

      searchState = 'done';
    } catch (err) {
      errorMsg = typeof err === 'string' ? err : 'Произошла ошибка при поиске';
      searchState = 'error';
    }
  }

  // --- Show all movies ---
  async function handleShowAll() {
    loadingAll = true;
    searchState = 'searching';
    errorMsg = '';
    results = [];

    try {
      const movies = await invoke<Movie[]>('get_all_movies_from_db');
      results = movies.map((movie, i) => ({ movie, rank: i + 1, reason: '' }));
      searchState = 'done';
    } catch (err) {
      errorMsg = typeof err === 'string' ? err : 'Не удалось загрузить фильмы';
      searchState = 'error';
    } finally {
      loadingAll = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleSearch();
    }
  }

  // Сбросить состояние при очистке поля
  $effect(() => {
    if (query === '') {
      searchState = 'idle';
      results = [];
      errorMsg = '';
    }
  });

  // Анимированный текст статуса
  const statusText: Record<'idle' | 'searching' | 'ranking' | 'done' | 'error', string> = {
    idle: '',
    searching: 'Ищем совпадения в базе...',
    ranking: 'Нейросеть анализирует результаты...',
    done: '',
    error: '',
  };
</script>

<!-- ============================================================
     Background decorations
     ============================================================ -->
<div class="bg-decorations" aria-hidden="true">
  <!-- Статичные угловые снежинки (атмосфера) -->
  <span class="snowflake snowflake-1">❄</span>
  <span class="snowflake snowflake-2">❄</span>
  <span class="snowflake snowflake-3">❅</span>
  <span class="snowflake snowflake-4">❆</span>
  <span class="snowflake snowflake-5">❄</span>
  <span class="snowflake snowflake-6">❅</span>

  <!-- Падающие снежинки (6 штук, стаггер через отрицательный delay) -->
  <span class="fall sf-1">❄</span>
  <span class="fall sf-2">❅</span>
  <span class="fall sf-3">❆</span>
  <span class="fall sf-4">❄</span>
  <span class="fall sf-5">❅</span>
  <span class="fall sf-6">❆</span>
</div>

<!-- ============================================================
     Settings button (top-right)
     ============================================================ -->
<div class="topbar">
  <button
    class="btn-icon settings-btn"
    onclick={() => (settingsOpen = true)}
    aria-label="Открыть настройки"
    title="Настройки"
  >
    <span class="settings-gear" aria-hidden="true">⚙</span>
  </button>
</div>

<!-- ============================================================
     Main content
     ============================================================ -->
<main class="main-content">
  <div class="page-container">

    <!-- Header / Hero -->
    <header class="app-header">
      <div class="star-wrap">
        <span class="hero-star" aria-hidden="true">★</span>
      </div>
      <h1 class="app-title">КиноЗапрос</h1>
      <p class="app-subtitle">
        Советские новогодние фильмы
        <br>
        По нечёткому запросу
        </p>
      <div class="header-ornament" aria-hidden="true">
        <span class="ornament-line"></span>
        <span class="ornament-diamond">◆</span>
        <span class="ornament-line"></span>
      </div>
    </header>

    <!-- Search bar -->
    <section class="search-section" aria-label="Поиск фильмов">
      <form class="search-form" onsubmit={handleSearch}>
        <div class="search-input-wrap">
          <!-- Search icon -->
          <span class="search-icon" aria-hidden="true">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="1.8"/>
              <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
            </svg>
          </span>
          <input
            bind:this={inputEl}
            bind:value={query}
            onkeydown={handleKeydown}
            type="search"
            class="search-input"
            placeholder="Ирония судьбы, советская комедия, Морозко..."
            aria-label="Поисковый запрос"
            autocomplete="off"
            autocorrect="off"
            spellcheck="false"
            disabled={searchState === 'searching' || searchState === 'ranking'}
          />
          <!-- Clear button -->
          {#if query}
            <button
              type="button"
              class="btn-icon search-clear"
              onclick={() => { query = ''; inputEl?.focus(); }}
              aria-label="Очистить поиск"
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                <path d="M2 2L14 14M14 2L2 14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
              </svg>
            </button>
          {/if}
        </div>

        <button
          type="submit"
          class="btn-primary search-btn"
          disabled={!query.trim() || searchState === 'searching' || searchState === 'ranking'}
          aria-label="Найти фильмы"
        >
          {#if searchState === 'searching' || searchState === 'ranking'}
            <span class="search-spinner" aria-hidden="true"></span>
          {:else}
            Найти
          {/if}
        </button>
      </form>

      <div class="toggle-row">
        <button
          type="button"
          class="btn-show-all"
          onclick={handleShowAll}
          disabled={loadingAll || searchState === 'searching' || searchState === 'ranking'}
          aria-label="Показать все фильмы"
        >
          {#if loadingAll}
            <span class="show-all-spinner" aria-hidden="true"></span>
          {:else}
            <span class="show-all-icon" aria-hidden="true">◈</span>
          {/if}
          Показать все фильмы
        </button>

        <label class="ai-toggle" title="Использовать ИИ для ранжирования результатов">
          <input type="checkbox" bind:checked={useAi} />
          <span class="ai-toggle-track">
            <span class="ai-toggle-thumb"></span>
          </span>
          <span class="ai-toggle-label">ИИ-ранжирование</span>
        </label>
      </div>
    </section>

    <!-- Status line -->
    {#if statusText[searchState]}
      <p class="status-line" role="status" aria-live="polite">
        <span class="status-dot" aria-hidden="true"></span>
        {statusText[searchState]}
      </p>
    {/if}

    <!-- Results / States -->
    <section class="results-section" aria-label="Результаты поиска" aria-live="polite">

      <!-- Idle state -->
      {#if searchState === 'idle'}
        <div class="empty-state">
          <div class="empty-icon" aria-hidden="true">
            <svg viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" width="80" height="80">
              <!-- Film strip -->
              <rect x="10" y="35" width="100" height="50" rx="6" stroke="var(--border-strong)" stroke-width="2"/>
              <rect x="10" y="35" width="16" height="50" fill="var(--bg-surface)" rx="4"/>
              <rect x="94" y="35" width="16" height="50" fill="var(--bg-surface)" rx="4"/>
              <!-- Sprocket holes -->
              <circle cx="18" cy="48" r="3" fill="var(--border-medium)"/>
              <circle cx="18" cy="60" r="3" fill="var(--border-medium)"/>
              <circle cx="18" cy="72" r="3" fill="var(--border-medium)"/>
              <circle cx="102" cy="48" r="3" fill="var(--border-medium)"/>
              <circle cx="102" cy="60" r="3" fill="var(--border-medium)"/>
              <circle cx="102" cy="72" r="3" fill="var(--border-medium)"/>
              <!-- Star in center -->
              <text x="60" y="68" text-anchor="middle" font-size="28" fill="var(--border-strong)">★</text>
              <!-- Snowflakes above -->
              <text x="30" y="22" text-anchor="middle" font-size="16" fill="var(--border-medium)" opacity="0.6">❄</text>
              <text x="60" y="18" text-anchor="middle" font-size="20" fill="var(--border-medium)" opacity="0.5">❆</text>
              <text x="90" y="22" text-anchor="middle" font-size="16" fill="var(--border-medium)" opacity="0.6">❅</text>
            </svg>
          </div>
          <p class="empty-title">Найдите советский новогодний фильм</p>
          <p class="empty-hint">
            Введите название, описание или имя актёра&nbsp;-<br />
            нейросеть поможет найти самое подходящее
          </p>
        </div>

      <!-- Loading states (handled by status-line above) -->
      {:else if searchState === 'searching' || searchState === 'ranking'}
        <div class="loading-state" aria-hidden="true">
          {#each Array(3) as _, i}
            <div class="skeleton-card" style="animation-delay: {i * 80}ms"></div>
          {/each}
        </div>

      <!-- Error state -->
      {:else if searchState === 'error'}
        <div class="error-state" role="alert">
          <span class="error-icon" aria-hidden="true">⚠</span>
          <p class="error-title">Что-то пошло не так</p>
          <p class="error-msg">{errorMsg}</p>
          <button class="btn-secondary retry-btn" onclick={() => handleSearch()}>
            Попробовать снова
          </button>
        </div>

      <!-- Results -->
      {:else if searchState === 'done'}
        {#if results.length === 0}
          <div class="no-results">
            <p class="no-results-icon" aria-hidden="true">☆</p>
            <p class="no-results-title">Ничего не найдено</p>
            <p class="no-results-hint">
              Попробуйте другой запрос или проверьте написание
            </p>
          </div>
        {:else}
          <div class="results-header">
            <p class="results-count">
              Найдено фильмов: <strong>{results.length}</strong>
            </p>
          </div>
          <ol class="results-list" aria-label="Список найденных фильмов">
            {#each results as item, i (item.movie.id)}
              <li>
                <MovieCard {item} index={i} onselect={(m) => (selectedMovie = m)} />
              </li>
            {/each}
          </ol>
        {/if}
      {/if}

    </section>

  </div>
</main>

<!-- Settings Modal -->
<SettingsModal bind:open={settingsOpen} onclose={() => (settingsOpen = false)} />

<!-- Movie Detail Modal -->
<MovieDetailModal movie={selectedMovie} onclose={() => (selectedMovie = null)} />

<style>
  /* ============================================================
     Background decorations
     ============================================================ */
  .bg-decorations {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 0;
    overflow: hidden;
  }

  .snowflake {
    position: absolute;
    color: var(--text-primary);
    opacity: 0.05;
    font-size: 48px;
    user-select: none;
  }

  .snowflake-1 { top: 8%;  left: 6%;   font-size: 36px; opacity: 0.14; }
  .snowflake-2 { top: 15%; right: 8%;  font-size: 52px; opacity: 0.11; }
  .snowflake-3 { top: 50%; left: 3%;   font-size: 28px; opacity: 0.13; }
  .snowflake-4 { top: 45%; right: 4%;  font-size: 40px; opacity: 0.12; }
  .snowflake-5 { bottom: 20%; left: 10%;  font-size: 60px; opacity: 0.09; }
  .snowflake-6 { bottom: 15%; right: 7%;  font-size: 44px; opacity: 0.10; }

  /* ============================================================
     Falling snowflakes
     ============================================================ */

  /*
   * Снежинка падает от -60px до 110vh с лёгким горизонтальным дрейфом.
   * Opacity: появляется плавно в начале, тает в конце.
   * Отрицательные animation-delay запускают каждую снежинку
   * в середине цикла → в первый же момент на экране 2-3 штуки,
   * а не 0 или сразу 6.
   */
  @keyframes snowfall {
    0%   { transform: translateY(-60px) translateX(0)    rotate(0deg);   opacity: 0; }
    6%   { opacity: 1; }
    50%  { transform: translateY(50vh)  translateX(18px)  rotate(120deg); }
    88%  { opacity: 0.8; }
    100% { transform: translateY(110vh) translateX(-8px)  rotate(240deg); opacity: 0; }
  }

  .fall {
    position: absolute;
    pointer-events: none;
    user-select: none;
    color: #fff;
    animation: snowfall linear infinite;
    will-change: transform, opacity;
  }

  /* Каждая снежинка: уникальная позиция, размер, скорость и сдвиг фазы */
  .sf-1 { left: 7%;   font-size: 20px; animation-duration: 13s; animation-delay:  -3s;  opacity: 0; }
  .sf-2 { left: 23%;  font-size: 14px; animation-duration: 17s; animation-delay:  -8s;  opacity: 0; }
  .sf-3 { left: 44%;  font-size: 22px; animation-duration: 11s; animation-delay:  -5s;  opacity: 0; }
  .sf-4 { left: 62%;  font-size: 16px; animation-duration: 15s; animation-delay: -12s;  opacity: 0; }
  .sf-5 { left: 79%;  font-size: 18px; animation-duration: 12s; animation-delay:  -1s;  opacity: 0; }
  .sf-6 { left: 91%;  font-size: 13px; animation-duration: 16s; animation-delay:  -9s;  opacity: 0; }

  /* ============================================================
     Topbar
     ============================================================ */
  .topbar {
    position: fixed;
    top: var(--space-4);
    right: var(--space-5);
    z-index: 100;
  }

  .settings-gear {
    font-size: 22px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    transform: translateY(-1px);
  }

  .settings-btn {
    width: 38px;
    height: 38px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    color: var(--text-secondary);
    border: 1px solid var(--border-medium);
    background: rgba(38, 16, 16, 0.85);
    backdrop-filter: blur(10px);
    transition:
      color var(--transition-base),
      border-color var(--transition-base),
      background-color var(--transition-base),
      box-shadow var(--transition-base);
  }

  .settings-btn:hover {
    color: var(--gold-300);
    border-color: var(--border-gold);
    background: var(--bg-card-hover);
    box-shadow: var(--glow-gold);
  }

  /* ============================================================
     Main content
     ============================================================ */
  .main-content {
    position: relative;
    z-index: 1;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    padding: var(--space-12) 0 var(--space-16);
  }

  /* ============================================================
     Header / Hero
     ============================================================ */
  .app-header {
    text-align: center;
    margin-bottom: var(--space-10);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
  }

  .star-wrap {
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hero-star {
    font-size: 32px;
    color: var(--red-500);
    display: block;
    animation: gentleFloat 4s ease-in-out infinite;
    filter: drop-shadow(0 0 6px rgba(204, 26, 26, 0.7));
  }

  .app-title {
    font-family: var(--font-display);
    font-size: 54px;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    line-height: 1;

    background: linear-gradient(
      120deg,
      #B8860B 0%,
      #D4A017 35%,
      #FFD700 47%,
      #FFFACD 50%,
      #FFD700 53%,
      #D4A017 65%,
      #B8860B 100%
    );
    background-size: 300% auto;
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    color: transparent;

    text-shadow: none;
    filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.5));

    animation: gold-shimmer 6s ease-in-out infinite;
  }

  @keyframes gold-shimmer {
    0%   { background-position: 150% center; }
    20%  { background-position: 150% center; }
    60%  { background-position: -150% center; }
    100% { background-position: -150% center; }
  }

  .app-subtitle {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    letter-spacing: 0.18em;
    text-transform: uppercase;
    font-weight: 400;
  }

  /* Ornament divider */
  .header-ornament {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--border-strong);
    margin-top: var(--space-1);
  }

  .ornament-line {
    display: block;
    width: 64px;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent,
      var(--red-600),
      transparent
    );
  }

  .ornament-diamond {
    font-size: 8px;
    color: var(--red-500);
    opacity: 1;
  }

  /* ============================================================
     Search section
     ============================================================ */
  .search-section {
    max-width: var(--search-max-width);
    margin: 0 auto var(--space-6);
    width: 100%;
  }

  .search-form {
    display: flex;
    gap: var(--space-3);
    align-items: stretch;
  }

  .search-input-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: var(--space-4);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    pointer-events: none;
    z-index: 1;
  }

  .search-input {
    height: 48px;
    padding-left: 46px;
    padding-right: 40px;
    font-size: var(--text-base);
    background: var(--bg-card);
    border: 1px solid var(--border-medium);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    transition:
      border-color var(--transition-base),
      box-shadow var(--transition-base),
      background-color var(--transition-base);
    outline: none;
    width: 100%;

    /* Remove default search clear button */
    -webkit-appearance: none;
    appearance: none;
  }

  .search-input::-webkit-search-cancel-button {
    display: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  .search-input:focus {
    border-color: var(--border-red);
    box-shadow: 0 0 0 3px rgba(163, 21, 21, 0.15), var(--shadow-sm);
    background: var(--bg-input);
  }

  .search-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .search-clear {
    position: absolute;
    right: var(--space-2);
    color: var(--text-muted);
    border: none;
    background: transparent;
    z-index: 1;
  }

  .search-clear:hover {
    color: var(--text-secondary);
    background: transparent;
    border: none;
  }

  .search-btn {
    height: 48px;
    padding: 0 var(--space-6);
    font-size: var(--text-base);
    border-radius: var(--radius-lg);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 100px;
    justify-content: center;
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Spinner in search button */
  .search-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(245, 240, 232, 0.3);
    border-top-color: var(--text-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  /* ============================================================
     Status line
     ============================================================ */
  .status-line {
    text-align: center;
    font-size: var(--text-sm);
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--red-600);
    animation: pulse 1.2s ease infinite;
  }

  /* ============================================================
     Results section
     ============================================================ */
  .results-section {
    max-width: var(--content-max-width);
    margin: 0 auto;
    width: 100%;
  }

  /* ---- Idle / Empty state ---- */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-16) 0;
    gap: var(--space-4);
    text-align: center;
  }

  .empty-icon {
    opacity: 0.35;
    margin-bottom: var(--space-2);
  }

  .empty-title {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 700;
    color: var(--text-secondary);
  }

  .empty-hint {
    font-size: var(--text-sm);
    color: var(--text-muted);
    line-height: 1.6;
  }

  /* ---- Skeleton loading ---- */
  .loading-state {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .skeleton-card {
    height: 140px;
    border-radius: var(--radius-lg);
    background: linear-gradient(
      90deg,
      var(--bg-card) 0%,
      rgba(122, 58, 58, 0.35) 50%,
      var(--bg-card) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s ease infinite;
    border: 1px solid var(--border-medium);
  }

  /* ---- Error state ---- */
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-12) 0;
    gap: var(--space-3);
    text-align: center;
  }

  .error-icon {
    font-size: 32px;
    color: var(--red-500);
  }

  .error-title {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    color: var(--text-secondary);
    font-weight: 700;
  }

  .error-msg {
    font-size: var(--text-sm);
    color: var(--text-muted);
    max-width: 400px;
  }

  .retry-btn {
    margin-top: var(--space-2);
  }

  /* ---- No results ---- */
  .no-results {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-12) 0;
    gap: var(--space-3);
    text-align: center;
  }

  .no-results-icon {
    font-size: 48px;
    color: var(--border-strong);
  }

  .no-results-title {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 700;
    color: var(--text-secondary);
  }

  .no-results-hint {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  /* ---- Results list ---- */
  .results-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    margin-bottom: var(--space-4);
    padding: 0 var(--space-1);
  }

  .results-count {
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .results-count strong {
    color: var(--text-secondary);
    font-weight: 600;
  }

  .results-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: 0;
    margin: 0;
  }

  .results-list li {
    display: block;
  }

  /* ============================================================
     Toggle row (show-all button + AI toggle)
     ============================================================ */
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: var(--space-2);
  }

  .btn-show-all {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: 28px;
    padding: 0 var(--space-3);
    font-size: var(--text-xs);
    font-weight: 500;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-medium);
    border-radius: var(--radius-full);
    transition:
      color var(--transition-base),
      border-color var(--transition-base),
      background-color var(--transition-base),
      box-shadow var(--transition-base);
  }

  .btn-show-all:hover:not(:disabled) {
    color: var(--gold-300);
    border-color: var(--border-gold);
    background: var(--bg-card-hover);
    box-shadow: var(--glow-gold);
  }

  .btn-show-all:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .show-all-icon {
    font-size: 10px;
    color: var(--gold-500);
    line-height: 1;
  }

  /* Spinner inside show-all button */
  .show-all-spinner {
    width: 11px;
    height: 11px;
    border: 1.5px solid rgba(245, 204, 69, 0.25);
    border-top-color: var(--gold-300);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }

  .ai-toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    cursor: pointer;
    user-select: none;
    justify-content: flex-end;
  }

  .ai-toggle input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .ai-toggle-track {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: var(--radius-full);
    background: var(--border-medium);
    border: 1px solid var(--border-subtle);
    transition: background var(--transition-base), border-color var(--transition-base);
    flex-shrink: 0;
  }

  .ai-toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: transform var(--transition-base), background var(--transition-base);
  }

  .ai-toggle input:checked + .ai-toggle-track {
    background: var(--red-700);
    border-color: var(--red-500);
  }

  .ai-toggle input:checked + .ai-toggle-track .ai-toggle-thumb {
    transform: translateX(16px);
    background: var(--gold-300);
  }

  .ai-toggle-label {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    letter-spacing: 0.03em;
  }
</style>
