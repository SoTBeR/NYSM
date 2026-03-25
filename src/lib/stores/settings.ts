import { writable } from 'svelte/store';
import type { AppSettings } from '$lib/types';

const defaultSettings: AppSettings = {
  ai_api_key: '',
  ai_base_url: '',
};

export const settingsStore = writable<AppSettings>(defaultSettings);
export const settingsLoaded = writable(false);
