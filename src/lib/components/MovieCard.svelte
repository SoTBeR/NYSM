<script lang="ts">
  import type { RankedMovie } from '$lib/types';

  interface Props {
    item: RankedMovie;
    /** Порядковый индекс для stagger-анимации */
    index?: number;
  }

  let { item, index = 0 }: Props = $props();

  /** Форматирует рейтинг как N.N */
  function formatRating(r: number | null): string {
    return r != null ? r.toFixed(1) : '—';
  }

  /** Возвращает класс цвета для рейтинга */
  function ratingClass(r: number | null): string {
    if (r == null) return 'rating-low';
    if (r >= 8) return 'rating-high';
    if (r >= 6.5) return 'rating-mid';
    return 'rating-low';
  }

  // Computed из props (реактивно через $derived)
  const movie = $derived(item.movie);
  const rank = $derived(item.rank);
  const reason = $derived(item.reason);
</script>

<article
  class="movie-card"
  style="animation-delay: {index * 60}ms"
  aria-label="{movie.title} ({movie.year})"
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
        <span class="rating {ratingClass(movie.rating)}" title="Рейтинг">
          <span class="rating-star" aria-hidden="true">★</span>
          {formatRating(movie.rating)}
        </span>
      </div>

      <div class="meta-row">
        <span class="meta-year">{movie.year}</span>
        {#if movie.duration_minutes}
          <span class="meta-sep" aria-hidden="true">·</span>
          <span class="meta-duration">{movie.duration_minutes} мин</span>
        {/if}
        {#if movie.director}
          <span class="meta-sep" aria-hidden="true">·</span>
          <span class="meta-director">реж. {movie.director}</span>
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

  .movie-card:hover {
    border-color: var(--border-gold);
    box-shadow: var(--shadow-md), var(--glow-gold);
    transform: translateY(-2px);
    background: var(--bg-card-hover);
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
    justify-content: space-between;
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

  /* ---- Rating ---- */
  .rating {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-sm);
    font-weight: 600;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .rating-star {
    font-size: 12px;
  }

  .rating-high { color: var(--gold-300); text-shadow: 0 0 8px rgba(245, 204, 69, 0.5); }
  .rating-mid  { color: var(--gold-400); }
  .rating-low  { color: var(--text-secondary); }

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
    text-transform: lowercase;
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
</style>
