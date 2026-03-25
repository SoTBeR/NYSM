// Типы данных, соответствующие Rust structs

export interface Movie {
  id: number;
  title: string;
  description: string;
  actors: string[];
  genres: string[];
  studios: string[];
  year: number;
  duration_minutes: number | null;
  director: string;
  rating: number | null;
}

export interface RankedMovie {
  movie: Movie;
  rank: number;
  reason: string;
}

export interface AppSettings {
  ai_api_key: string;
  ai_base_url: string;
}

// UI-специфичные типы

export type SearchState =
  | { type: 'idle' }
  | { type: 'loading' }
  | { type: 'results'; items: RankedMovie[] }
  | { type: 'error'; message: string };
