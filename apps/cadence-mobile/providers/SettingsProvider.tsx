import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import AsyncStorage from "@react-native-async-storage/async-storage";

export interface Settings {
    /** Desktop volume change per hardware volume-button press, in percent (1..100). */
    volumeStepPercent: number;
}

export const DEFAULT_SETTINGS: Settings = { volumeStepPercent: 5 };
export const VOLUME_STEP_MIN = 1;
export const VOLUME_STEP_MAX = 100;

const STORAGE_KEY = "cadence.settings";

interface SettingsContextValue {
    settings: Settings;
    update: (patch: Partial<Settings>) => void;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

/** Keeps only known keys with valid values, so a stale or corrupt store falls back to defaults. */
function sanitize(raw: unknown): Settings {
    const out = { ...DEFAULT_SETTINGS };
    if (raw && typeof raw === "object") {
        const step = (raw as Record<string, unknown>).volumeStepPercent;
        if (typeof step === "number" && Number.isFinite(step)) {
            out.volumeStepPercent = Math.min(VOLUME_STEP_MAX, Math.max(VOLUME_STEP_MIN, Math.round(step)));
        }
    }
    return out;
}

/** App preferences, persisted to AsyncStorage. Defaults apply until the stored copy loads. */
export function SettingsProvider({ children }: { children: ReactNode }) {
    const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

    useEffect(() => {
        AsyncStorage.getItem(STORAGE_KEY)
            .then((json) => {
                if (json) setSettings(sanitize(JSON.parse(json)));
            })
            .catch(() => {});
    }, []);

    const update = useCallback((patch: Partial<Settings>) => {
        setSettings((prev) => {
            const next = sanitize({ ...prev, ...patch });
            AsyncStorage.setItem(STORAGE_KEY, JSON.stringify(next)).catch(() => {});
            return next;
        });
    }, []);

    const value = useMemo(() => ({ settings, update }), [settings, update]);
    return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings() {
    const ctx = useContext(SettingsContext);
    if (!ctx) throw new Error("useSettings must be used inside <SettingsProvider>");
    return ctx;
}
