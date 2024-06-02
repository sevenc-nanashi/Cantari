export type Settings = {
  paths: string[];
  ongen_limit: number;
  ongen_settings: Record<string, OngenSettings>;
};

export type OngenSettings = {
  name: string | null;
  portrait: string | null;
  style_settings: StyleSettings[];
};

export type StyleSettings = {
  name: string;
  portrait: string | null;
  icon: string | null;

  whisper: boolean;
  formant_shift: number;
  breathiness: number;
  tension: number;
  peak_compression: number;
  voicing: number;
};

export type Ongen = {
  name: string;
};

const createUse =
  <T>(key: string) =>
  (): T => {
    const el = document.querySelector(`script#${key}`) as HTMLScriptElement;
    const json = el.innerHTML;
    return JSON.parse(json) as T;
  };

export const useSettings = createUse<Settings>("settings");

export const useOngens = createUse<Record<string, Ongen>>("ongens");
