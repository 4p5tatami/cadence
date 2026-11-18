import { useCallback, useEffect, useRef, useState } from "react";

export interface TrackRecord {
    id: number;
    path: string;
    title: string;
    artist: string;
    duration_ms: number;
}

export type PlayerMode = "Default" | "Shuffle" | "Replay";

export interface PlaybackState {
    trackPath: string;
    title: string | null;
    artist: string | null;
    durationMs: number;
    positionMs: number;
    playing: boolean;
    snapshotAtMs: number;
    mode: PlayerMode;
}

type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

/** A command the desktop rejected. `id` changes on every error so repeats still notify. */
export interface DesktopError {
    id: number;
    message: string;
}

const BACKOFF_INITIAL_MS = 1_000;
const BACKOFF_MAX_MS = 16_000;

export function useDesktopSync(url: string | null) {
    const wsRef = useRef<WebSocket | null>(null);

    const [status, setStatus] = useState<ConnectionStatus>("disconnected");
    const [playback, setPlayback] = useState<PlaybackState | null>(null);
    const [searchResults, setSearchResults] = useState<TrackRecord[]>([]);
    /** Desktop volume level in 0..1 (slider position, not linear gain). Null until the desktop reports it. */
    const [volume, setVolumeState] = useState<number | null>(null);
    const [lastError, setLastError] = useState<DesktopError | null>(null);

    const reportError = useCallback((message: string) => {
        setLastError((prev) => ({ id: (prev?.id ?? 0) + 1, message }));
    }, []);

    useEffect(() => {
        setPlayback(null);
        setSearchResults([]);
        setVolumeState(null);

        if (!url) {
            setStatus("disconnected");
            return;
        }

        // Per-connection state. Kept local (not in refs) so a socket from a previous
        // url that closes late can't schedule retries against the new connection.
        let cancelled = false;
        let everOpened = false;
        let backoffMs = BACKOFF_INITIAL_MS;
        let retryTimer: ReturnType<typeof setTimeout> | undefined;

        function connect() {
            if (cancelled) return;

            setStatus("connecting");
            const ws = new WebSocket(url!);
            wsRef.current = ws;

            ws.onopen = () => {
                if (cancelled) return;
                everOpened = true;
                backoffMs = BACKOFF_INITIAL_MS; // reset on success
                setStatus("connected");
            };

            ws.onerror = () => {
                // onclose will fire immediately after - handled there
            };

            ws.onclose = () => {
                if (cancelled) return;
                if (wsRef.current === ws) wsRef.current = null;
                setPlayback(null);

                // The first attempt failing means the device is unreachable: report it.
                // After a successful connection, keep retrying (e.g. while the desktop
                // restarts) until the backoff reaches its cap.
                if (!everOpened || backoffMs > BACKOFF_MAX_MS) {
                    setStatus("error");
                    return;
                }

                setStatus("connecting");
                retryTimer = setTimeout(connect, backoffMs);
                backoffMs *= 2;
            };

            ws.onmessage = (e) => {
                try {
                    const msg = JSON.parse(e.data as string);
                    // Sent with both "state" and "stopped"; React skips the re-render when unchanged.
                    if (typeof msg.volume === "number") setVolumeState(msg.volume);
                    if (msg.type === "state") {
                        setPlayback({
                            trackPath: msg.track_path,
                            title: msg.title ?? null,
                            artist: msg.artist ?? null,
                            durationMs: msg.duration_ms,
                            positionMs: msg.position_ms,
                            playing: msg.playing,
                            snapshotAtMs: Date.now(), // use client receive time to avoid PC/phone clock skew
                            mode: msg.mode ?? "Default",
                        });
                    } else if (msg.type === "stopped") {
                        setPlayback(null);
                    } else if (msg.type === "search_results") {
                        setSearchResults(msg.tracks ?? []);
                    } else if (msg.type === "error") {
                        reportError(String(msg.message));
                    }
                } catch {}
            };
        }

        connect();

        return () => {
            cancelled = true;
            clearTimeout(retryTimer);
            wsRef.current?.close();
            wsRef.current = null;
        };
    }, [url, reportError]);

    /** Sends a command; reports an error instead of silently dropping it when offline. */
    const send = useCallback((obj: object) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify(obj));
            return true;
        }
        reportError("Not connected to Cadence desktop.");
        return false;
    }, [reportError]);

    const search = useCallback((query: string) => {
        if (query.trim()) {
            send({ type: "search", query });
        } else {
            setSearchResults([]);
        }
    }, [send]);

    const play = useCallback((path: string) => send({ type: "play", path }), [send]);
    const pause = useCallback(() => send({ type: "pause" }), [send]);
    const resume = useCallback(() => send({ type: "resume" }), [send]);
    const stop = useCallback(() => send({ type: "stop" }), [send]);
    const next = useCallback(() => send({ type: "next" }), [send]);
    const previous = useCallback(() => send({ type: "previous" }), [send]);
    const seek = useCallback((toMs: number) => send({ type: "seek", to_ms: toMs }), [send]);
    const setMode = useCallback((mode: PlayerMode) => send({ type: "set_mode", mode }), [send]);
    /** Sets the desktop volume level (0..1). The desktop clamps and applies a perceptual curve. */
    const setVolume = useCallback(
        (level: number) => send({ type: "set_volume", volume: Math.min(1, Math.max(0, level)) }),
        [send],
    );

    return {
        status, playback, searchResults, volume, lastError,
        search, play, pause, resume, stop, next, previous, seek, setMode, setVolume,
    };
}
