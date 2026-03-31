<script lang="ts">
  import type { RankedMovie } from '$lib/types';

  interface Props {
    item: RankedMovie;
    /** Порядковый индекс для stagger-анимации */
    index?: number;
    /** Вызывается при клике на карточку */
    onselect?: (item: RankedMovie) => void;
  }

  let { item, index = 0, onselect }: Props = $props();

  // Computed из props (реактивно через $derived)
  const movie = $derived(item.movie);
  const rank = $derived(item.rank);
  const reason = $derived(item.reason);

  function formatDuration(mins: number | null): string {
    if (!mins) return '—';
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    if (h === 0) return `${m} мин`;
    return m === 0 ? `${h} ч` : `${h} ч ${m} мин`;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<article
  class="movie-card"
  class:clickable={!!onselect}
  style="animation-delay: {index * 60}ms"
  aria-label="{movie.title} ({movie.year})"
  role={onselect ? 'button' : undefined}
  tabindex={onselect ? 0 : undefined}
  onclick={() => onselect?.(item)}
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect?.(item); } }}
>
  <!-- Rank badge -->
  <div class="rank-badge" aria-label="Позиция {rank}">
    {rank}
  </div>

  <!-- Content -->
  <div class="card-content">
    <div class="card-header">
      <div class="title-row">
        <h3 class="movie-title">{movie.title}</h3>
      </div>

      <div class="meta-row">
        <span class="meta-year">{movie.year}</span>
        {#if movie.duration_minutes}
          <span class="meta-sep" aria-hidden="true">·</span>
          <span class="meta-duration">{formatDuration(movie.duration_minutes)}</span>
        {/if}
        {#if movie.director}
          <span class="meta-sep" aria-hidden="true">·</span>
          <span class="meta-director">{movie.director}</span>
        {/if}
        {#if movie.studios.length > 0}
          <span class="meta-sep" aria-hidden="true">·</span>
          <span class="meta-studio">{movie.studios[0]}</span>
        {/if}
      </div>
    </div>

    {#if movie.genres.length > 0}
      <div class="genres" role="list" aria-label="Жанры">
        {#each movie.genres as genre}
          <span class="genre-tag" role="listitem">{genre}</span>
        {/each}
      </div>
    {/if}

    {#if movie.description}
      <p class="description truncate-3">{movie.description}</p>
    {/if}

    {#if movie.actors.length > 0}
      <p class="actors">
        <span class="actors-label">В ролях:</span>
        {movie.actors.slice(0, 3).join(', ')}{movie.actors.length > 3 ? ' и др.' : ''}
      </p>
    {/if}

    {#if reason}
      <p class="ai-reason">
        <span class="ai-label" aria-label="Объяснение от AI">AI:</span>
        {reason}
      </p>
    {/if}
  </div>
</article>

<style>
  .movie-card {
    display: flex;
    gap: var(--space-5);
    padding: var(--space-5);
    background: var(--bg-card);
    border: 1px solid var(--border-medium);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
    position: relative;
    overflow: hidden;
    cursor: default;

    animation: fadeSlideUp var(--transition-appear) both;
    transition:
      border-color var(--transition-base),
      box-shadow var(--transition-base),
      transform var(--transition-base),
      background-color var(--transition-base);
  }

  .movie-card::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      rgba(204, 26, 26, 0.07) 0%,
      transparent 55%
    );
    pointer-events: none;
  }

  .movie-card.clickable {
    cursor: pointer;
  }

  .movie-card.clickable:hover {
    border-color: var(--border-gold);
    box-shadow: var(--shadow-md), var(--glow-gold);
    transform: translateY(-2px);
    background: var(--bg-card-hover);
  }

  .movie-card.clickable:focus-visible {
    outline: 2px solid var(--red-500);
    outline-offset: 2px;
  }

  /* ---- Rank badge ---- */
  .rank-badge {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    width: 28px;
    height: 28px;
    border-radius: var(--radius-full);
    background: linear-gradient(160deg, var(--red-700) 0%, var(--red-900) 100%);
    border: 1px solid var(--red-500);
    color: var(--gold-200);
    font-size: var(--text-xs);
    font-weight: 700;
    font-family: var(--font-body);
    display: flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    letter-spacing: 0;
    box-shadow: 0 0 8px rgba(204, 26, 26, 0.4);
  }

  /* ---- Content ---- */
  .card-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-right: var(--space-8); /* space for rank badge */
  }

  .card-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .movie-title {
    font-family: var(--font-display);
    font-size: var(--text-md);
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.3;
    flex: 1;
    min-width: 0;
  }

  /* ---- Meta ---- */
  .meta-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .meta-year {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--red-500);
  }

  .meta-sep {
    color: var(--text-muted);
  }

  .meta-duration {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .meta-director {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    font-style: italic;
  }

  .meta-studio {
    font-size: var(--text-sm);
    color: var(--text-muted);
    font-style: italic;
    text-transform: uppercase;
  }

  /* ---- Genres ---- */
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .genre-tag {
    font-size: var(--text-xs);
    font-weight: 500;
    color: var(--gold-200);
    background: rgba(232, 184, 48, 0.14);
    border: 1px solid rgba(232, 184, 48, 0.35);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-2);
    letter-spacing: 0.01em;
  }

  /* ---- Description ---- */
  .description {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.55;
  }

  /* ---- Actors ---- */
  .actors {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .actors-label {
    color: var(--text-primary);
    font-weight: 500;
    margin-right: 3px;
  }

  /* ---- AI reason ---- */
  .ai-reason {
    font-size: var(--text-xs);
    color: var(--text-secondary);
    font-style: italic;
    border-left: 2px solid var(--red-600);
    padding-left: var(--space-2);
    margin-top: var(--space-1);
  }

  .ai-label {
    color: var(--red-400);
    font-weight: 600;
    font-style: normal;
    margin-right: 4px;
  }

  /* ============================================================
     Mobile — ≤ 600px
     ============================================================ */
  @media (max-width: 600px) {
    .movie-card {
      padding: var(--space-3);
      gap: var(--space-3);
    }

    /* Genres already flex-wrap — no change needed; confirmed above */

    /* Less right padding since rank badge is the same size */
    .card-content {
      padding-right: var(--space-6);
    }

    .movie-title {
      font-size: var(--text-base);
    }

    .ai-reason {
      padding-left: var(--space-2);
    }
  }
</style>
