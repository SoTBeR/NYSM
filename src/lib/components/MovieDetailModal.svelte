<script lang="ts">
  import type { RankedMovie } from '$lib/types';

  interface Props {
    movie: RankedMovie | null;
    onclose: () => void;
  }

  let { movie, onclose }: Props = $props();

  const open = $derived(movie !== null);
  const m = $derived(movie?.movie ?? null);

  function formatDuration(mins: number | null): string {
    if (!mins) return '—';
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    if (h === 0) return `${m} мин`;
    return m === 0 ? `${h} ч` : `${h} ч ${m} мин`;
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

{#if open && m}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="detail-title"
    tabindex="-1"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div class="modal-panel">

      <!-- Header -->
      <div class="modal-header">
        <div class="header-meta">
          <span class="header-year">{m.year}</span>
          {#if m.duration_minutes}
            <span class="header-sep" aria-hidden="true">·</span>
            <span class="header-duration">{formatDuration(m.duration_minutes)}</span>
          {/if}
        </div>

        <div class="header-title-row">
          <h2 id="detail-title" class="modal-title">{m.title}</h2>
        </div>

        <button
          class="btn-icon modal-close"
          onclick={onclose}
          aria-label="Закрыть"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M2 2L14 14M14 2L2 14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- Divider -->
      <div class="modal-divider" aria-hidden="true"></div>

      <!-- Scrollable body -->
      <div class="modal-body">

        <!-- Genres -->
        {#if m.genres.length > 0}
          <div class="detail-section">
            <div class="genres" role="list" aria-label="Жанры">
              {#each m.genres as genre}
                <span class="genre-tag" role="listitem">{genre}</span>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Description -->
        {#if m.description}
          <div class="detail-section">
            <p class="section-label">О фильме</p>
            <p class="description">{m.description}</p>
          </div>
        {/if}

        <!-- Director + Studio row -->
        <div class="detail-grid">
          {#if m.director}
            <div class="detail-cell">
              <p class="section-label">Режиссёр</p>
              <p class="cell-value">{m.director}</p>
            </div>
          {/if}
          {#if m.studios.length > 0}
            <div class="detail-cell">
              <p class="section-label">Студия</p>
              <p class="cell-value cell-value--studio">{m.studios.join(', ')}</p>
            </div>
          {/if}
        </div>

        <!-- Actors -->
        {#if m.actors.length > 0}
          <div class="detail-section">
            <p class="section-label">В ролях</p>
            <div class="actors-list">
              {#each m.actors as actor}
                <span class="actor-chip">{actor}</span>
              {/each}
            </div>
          </div>
        {/if}

        <!-- AI reason -->
        {#if movie?.reason}
          <div class="detail-section ai-section">
            <p class="section-label ai-section-label">
              <span class="ai-dot" aria-hidden="true"></span>
              Почему AI выбрал этот фильм
            </p>
            <p class="ai-reason">{movie.reason}</p>
          </div>
        {/if}

      </div>
      <!-- end modal-body -->

      <!-- Footer: rank badge -->
      {#if (movie?.rank ?? 0) > 0}
        <div class="modal-footer">
          <span class="rank-label">Позиция в выдаче</span>
          <span class="rank-badge">#{movie?.rank}</span>
        </div>
      {/if}

    </div>
  </div>
{/if}

<style>
  /* ---- Overlay ---- */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: var(--space-6);
    animation: overlayAppear var(--transition-base) ease;
  }

  /* ---- Panel ---- */
  .modal-panel {
    background: var(--bg-modal);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    width: 600px;
    max-width: 100%;
    max-height: calc(100vh - var(--space-12));
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-xl), 0 0 48px rgba(180, 20, 20, 0.18);
    animation: modalAppear var(--transition-appear) both;
    overflow: hidden;
    position: relative;
  }

  /* ---- Header ---- */
  .modal-header {
    padding: var(--space-6) var(--space-6) var(--space-5);
    position: relative;
    flex-shrink: 0;
  }

  .modal-header::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(204, 26, 26, 0.08) 0%, transparent 60%);
    pointer-events: none;
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .header-year {
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--red-500);
    letter-spacing: 0.04em;
  }

  .header-sep {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .header-duration {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  .header-title-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding-right: var(--space-8); /* room for close button */
  }

  .modal-title {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.25;
    flex: 1;
    min-width: 0;
  }

  .modal-close {
    position: absolute;
    top: var(--space-4);
    right: var(--space-4);
    color: var(--text-muted);
    z-index: 1;
  }

  .modal-close:hover {
    color: var(--text-secondary);
  }

  /* ---- Divider ---- */
  .modal-divider {
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--red-800), transparent);
    margin: 0 var(--space-6);
    flex-shrink: 0;
  }

  /* ---- Body (scrollable) ---- */
  .modal-body {
    padding: var(--space-6);
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-height: 0;
  }

  /* ---- Sections ---- */
  .detail-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .section-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  /* ---- Description ---- */
  .description {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    line-height: 1.65;
  }

  /* ---- Director + Studio grid ---- */
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-4) var(--space-6);
  }

  .detail-cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .cell-value {
    font-size: var(--text-sm);
    color: var(--text-primary);
    font-weight: 400;
    line-height: 1.45;
  }

  .cell-value--studio {
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
    padding: 3px var(--space-2);
    letter-spacing: 0.01em;
  }

  /* ---- Actors ---- */
  .actors-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .actor-chip {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--border-medium);
    border-radius: var(--radius-md);
    padding: 4px var(--space-3);
    line-height: 1.3;
    transition: border-color var(--transition-base), color var(--transition-base);
  }

  /* ---- AI reason ---- */
  .ai-section {
    background: rgba(163, 21, 21, 0.07);
    border: 1px solid rgba(163, 21, 21, 0.22);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }

  .ai-section-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .ai-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--red-500);
    flex-shrink: 0;
    box-shadow: 0 0 6px rgba(204, 26, 26, 0.6);
  }

  .ai-reason {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    font-style: italic;
    line-height: 1.6;
  }

  /* ---- Footer ---- */
  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border-medium);
    flex-shrink: 0;
  }

  .rank-label {
    font-size: var(--text-xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .rank-badge {
    font-size: var(--text-sm);
    font-weight: 700;
    color: var(--gold-200);
    background: linear-gradient(160deg, var(--red-700) 0%, var(--red-900) 100%);
    border: 1px solid var(--red-500);
    border-radius: var(--radius-full);
    padding: 2px var(--space-3);
    box-shadow: 0 0 8px rgba(204, 26, 26, 0.35);
  }
</style>
