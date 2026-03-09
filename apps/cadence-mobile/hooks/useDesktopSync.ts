import { useCallback, useEffect, useRef, useState } from "react";

export interface TrackRecord {
    id: number;
    path: string;
    title: string;
    artist: string;
    duration_ms: number;
}

export interface PlaybackState {
    trackPath: string;
    title: string | null;
    artist: string | null;
    durationMs: number;
    positionMs: number;
    playing: boolean;
    snapshotAtMs: number;
}

type ConnectionStatus = "disconnected" | "connecting" | "connected" | "error";

export function useDesktopSync(url: string | null) {
    const wsRef = useRef<WebSocket | null>(null);
    const [status, setStatus] = useState<ConnectionStatus>("disconnected");
    const [playback, setPlayback] = useState<PlaybackState | null>(null);
    const [searchResults, setSearchResults] = useState<TrackRecord[]>([]);

    useEffect(() => {
        if (!url) return;

        setStatus("connecting");
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onopen = () => setStatus("connected");
        ws.onerror = () => setStatus("error");
        ws.onclose = () => {
            setStatus("disconnected");
            setPlayback(null);
            wsRef.current = null;
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
                    });
                } else if (msg.type === "stopped") {
                    setPlayback(null);
                } else if (msg.type === "search_results") {
                    setSearchResults(msg.tracks ?? []);
                }
            } catch {}
        };

        return () => {
            ws.close();
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
    const seek = useCallback((toMs: number) => send({ type: "seek", to_ms: toMs }), [send]);

    return { status, playback, searchResults, search, play, pause, resume, stop, seek };
}
