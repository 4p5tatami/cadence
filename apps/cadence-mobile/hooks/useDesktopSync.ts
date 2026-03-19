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

const BACKOFF_INITIAL_MS = 1_000;
const BACKOFF_MAX_MS = 16_000;

export function useDesktopSync(url: string | null) {
    const wsRef = useRef<WebSocket | null>(null);
    const backoffRef = useRef(BACKOFF_INITIAL_MS);
    const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const cancelledRef = useRef(false); // true when url changes / component unmounts

    const [status, setStatus] = useState<ConnectionStatus>("disconnected");
    const [playback, setPlayback] = useState<PlaybackState | null>(null);
    const [searchResults, setSearchResults] = useState<TrackRecord[]>([]);

    useEffect(() => {
        if (!url) {
            // User explicitly disconnected - clean up and stop retrying.
            cancelledRef.current = true;
            clearTimeout(retryTimerRef.current ?? undefined);
            wsRef.current?.close();
            wsRef.current = null;
            setStatus("disconnected");
            setPlayback(null);
            return;
        }

        cancelledRef.current = false;
        backoffRef.current = BACKOFF_INITIAL_MS;

        function connect() {
            if (cancelledRef.current) return;

            setStatus("connecting");
            const ws = new WebSocket(url!);
            wsRef.current = ws;
            let opened = false;

            ws.onopen = () => {
                opened = true;
                backoffRef.current = BACKOFF_INITIAL_MS; // reset on success
                setStatus("connected");
            };

            ws.onerror = () => {
                // onclose will fire immediately after - handled there
            };

            ws.onclose = () => {
                wsRef.current = null;
                setPlayback(null);

                if (cancelledRef.current) {
                    setStatus("disconnected");
                    return;
                }

                if (!opened || backoffRef.current >= BACKOFF_MAX_MS) {
                    // never connected or backoff time exceeded threshold - just stop trying
                    setStatus("error");
                    return;
                }

                setStatus("connecting");
                retryTimerRef.current =
                    setTimeout(() => {
                        backoffRef.current = Math.min(backoffRef.current * 2, BACKOFF_MAX_MS);
                        connect();
                    }, backoffRef.current);
            };

            ws.onmessage = (e) => {
                try {
                    const msg = JSON.parse(e.data as string);
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
                    }
                } catch {}
            };
        }

        connect();

        return () => {
            cancelledRef.current = true;
            clearTimeout(retryTimerRef.current ?? undefined);
            wsRef.current?.close();
            wsRef.current = null;
        };
    }, [url]);

    const send = useCallback((obj: object) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify(obj));
        }
    }, []);

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

    return { status, playback, searchResults, search, play, pause, resume, stop, next, previous, seek, setMode };
}
