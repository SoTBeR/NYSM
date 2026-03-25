<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { settingsStore } from '$lib/stores/settings';
  import type { AppSettings } from '$lib/types';

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open = $bindable(), onclose }: Props = $props();

  // Локальная копия настроек для редактирования
  let localApiKey = $state('');
  let localBaseUrl = $state('');
  let showApiKey = $state(false);
  let saving = $state(false);
  let error = $state('');

  // Синхронизируем локальное состояние когда модалка открывается
  $effect(() => {
    if (open) {
      const unsub = settingsStore.subscribe((s) => {
        localApiKey = s.ai_api_key;
        localBaseUrl = s.ai_base_url;
      });
      unsub();
      error = '';
    }
  });

  async function handleSave() {
    saving = true;
    error = '';
    try {
      const settings: AppSettings = {
        ai_api_key: localApiKey.trim(),
        ai_base_url: localBaseUrl.trim(),
      };
      await invoke<void>('save_settings', { settings });
      settingsStore.set(settings);
      onclose();
    } catch (e) {
      error = typeof e === 'string' ? e : 'Не удалось сохранить настройки';
    } finally {
      saving = false;
    }
  }

  function handleCancel() {
    error = '';
    onclose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleCancel();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div class="modal-panel">
      <!-- Header -->
      <div class="modal-header">
        <h2 id="settings-title" class="modal-title">
          <span class="title-star" aria-hidden="true">★</span>
          Настройки
        </h2>
        <button
          class="btn-icon modal-close"
          onclick={handleCancel}
          aria-label="Закрыть настройки"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M2 2L14 14M14 2L2 14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- Divider -->
      <div class="modal-divider" aria-hidden="true"></div>

      <!-- Body -->
      <div class="modal-body">
        <!-- API Key field -->
        <div class="field-group">
          <label for="api-key-input" class="field-label">
            API ключ нейросети
          </label>
          <div class="input-wrap">
            <input
              id="api-key-input"
              type={showApiKey ? 'text' : 'password'}
              class="field-input"
              bind:value={localApiKey}
              placeholder="Введите API ключ..."
              autocomplete="off"
              spellcheck="false"
            />
            <button
              type="button"
              class="btn-icon toggle-visibility"
              onclick={() => (showApiKey = !showApiKey)}
              aria-label={showApiKey ? 'Скрыть ключ' : 'Показать ключ'}
              title={showApiKey ? 'Скрыть' : 'Показать'}
            >
              {#if showApiKey}
                <!-- Eye-off icon -->
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                  <line x1="1" y1="1" x2="23" y2="23" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
                </svg>
              {:else}
                <!-- Eye icon -->
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.8"/>
                  <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8"/>
                </svg>
              {/if}
            </button>
          </div>
          <p class="field-hint">
            Ключ используется для ранжирования результатов поиска через нейросеть.
          </p>
        </div>

        <!-- Base URL field -->
        <div class="field-group">
          <label for="base-url-input" class="field-label">
            URL API <span class="field-optional">(необязательно)</span>
          </label>
          <input
            id="base-url-input"
            type="text"
            class="field-input"
            bind:value={localBaseUrl}
            placeholder="https://api.example.com"
            autocomplete="off"
            spellcheck="false"
          />
          <p class="field-hint">
            Оставьте пустым для использования стандартного адреса агрегатора.
          </p>
        </div>

        <!-- Error message -->
        {#if error}
          <div class="error-msg" role="alert">
            <span aria-hidden="true">⚠</span> {error}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="modal-footer">
        <button
          type="button"
          class="btn-secondary"
          onclick={handleCancel}
          disabled={saving}
        >
          Отмена
        </button>
        <button
          type="button"
          class="btn-primary"
          onclick={handleSave}
          disabled={saving}
        >
          {#if saving}
            <span class="saving-dots" aria-label="Сохранение...">
              <span></span><span></span><span></span>
            </span>
          {:else}
            Сохранить
          {/if}
        </button>
      </div>
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
    animation: overlayAppear var(--transition-base) ease;
  }

  /* ---- Panel ---- */
  .modal-panel {
    background: var(--bg-modal);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xl);
    width: 420px;
    max-width: calc(100vw - var(--space-8));
    box-shadow: var(--shadow-xl), 0 0 40px rgba(180, 20, 20, 0.2);
    animation: modalAppear var(--transition-appear) both;
    overflow: hidden;
  }

  /* ---- Header ---- */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-5) var(--space-6);
  }

  .modal-title {
    font-family: var(--font-display);
    font-size: var(--text-lg);
    font-weight: 700;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .title-star {
    color: var(--red-500);
    font-size: 18px;
    filter: drop-shadow(0 0 6px rgba(204, 26, 26, 0.7));
  }

  .modal-close {
    color: var(--text-muted);
  }
  .modal-close:hover {
    color: var(--text-secondary);
  }

  /* ---- Divider ---- */
  .modal-divider {
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--red-800), transparent);
    margin: 0 var(--space-6);
  }

  /* ---- Body ---- */
  .modal-body {
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  /* ---- Fields ---- */
  .field-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field-label {
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.01em;
  }

  .field-optional {
    font-weight: 400;
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .field-input {
    background: var(--bg-input);
    border: 1px solid var(--border-medium);
    color: var(--text-primary);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    font-size: var(--text-base);
    font-family: var(--font-body);
    transition:
      border-color var(--transition-base),
      box-shadow var(--transition-base);
    outline: none;
    width: 100%;
  }

  .field-input::placeholder {
    color: var(--text-muted);
  }

  .field-input:focus {
    border-color: var(--border-red);
    box-shadow: 0 0 0 3px rgba(163, 21, 21, 0.15);
  }

  .field-hint {
    font-size: var(--text-xs);
    color: var(--text-muted);
    line-height: 1.5;
  }

  .input-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-wrap .field-input {
    padding-right: 44px;
  }

  .toggle-visibility {
    position: absolute;
    right: var(--space-2);
    color: var(--text-muted);
    border: none;
    background: transparent;
  }

  .toggle-visibility:hover {
    color: var(--text-secondary);
    background: transparent;
    border: none;
  }

  /* ---- Error ---- */
  .error-msg {
    font-size: var(--text-sm);
    color: var(--red-400);
    background: rgba(163, 21, 21, 0.1);
    border: 1px solid rgba(163, 21, 21, 0.25);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  /* ---- Footer ---- */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-3);
    padding: var(--space-5) var(--space-6);
    border-top: 1px solid var(--border-medium);
  }

  /* ---- Saving dots ---- */
  .saving-dots {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }

  .saving-dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--gold-300);
    animation: pulse 1.2s ease infinite;
  }
  .saving-dots span:nth-child(2) { animation-delay: 0.2s; }
  .saving-dots span:nth-child(3) { animation-delay: 0.4s; }
</style>
