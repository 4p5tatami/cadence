import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StatusResponse {
    path: string;
    duration_ms: number;
    position_ms: number;
    paused: boolean;
    title: string | null;
    artist: string | null;
}

export function usePlayback() {
    // Anchor for extrapolation — written by poll, read by rAF. Never in state.
    const startRef = useRef({ positionMs: 0, wallClock: 0, playing: false });
    // Non-null while the user is dragging the thumb.
    const dragRef = useRef<number | null>(null);
    // rAF handle for cleanup.
    const rafRef = useRef<number>(0);

    const [displayMs, setDisplayMs] = useState(0);
    const [durationMs, setDurationMs] = useState(0);
    const [isPaused, setIsPaused] = useState(true);
    const [isAnyTrackActive, setIsAnyTrackActive] = useState(false);
    const [trackPath, setTrackPath] = useState<string | null>(null);
    const [trackTitle, setTrackTitle] = useState<string | null>(null);
    const [trackArtist, setTrackArtist] = useState<string | null>(null);

    // Polls backend and resets the extrapolation anchor.
    const poll = useCallback(async () => {
        const status = await invoke<StatusResponse | null>("status");
        if (status === null) {
            if (dragRef.current === null) {
                startRef.current = { positionMs: 0, wallClock: Date.now(), playing: false };
            }
            setIsPaused(true);
            setIsAnyTrackActive(false);
            setTrackPath(null);
            setTrackTitle(null);
            setTrackArtist(null);
            setDurationMs(0);
        } else {
            if (dragRef.current === null) {
                startRef.current = {
                    positionMs: status.position_ms,
                    wallClock: Date.now(),
                    playing: !status.paused,
                };
            }
            setIsPaused(status.paused);
            setIsAnyTrackActive(true);
            setTrackPath(status.path);
            setTrackTitle(status.title);
            setTrackArtist(status.artist);
            setDurationMs(status.duration_ms);
        }
    }, []);

    // Poll every second.
    useEffect(() => {
        void poll();
        const id = setInterval(() => { void poll(); }, 1000);
        return () => clearInterval(id);
    }, [poll]);

    // rAF loop — extrapolates forward from the last anchor at 60fps.
    const tick = useCallback(() => {
        if (dragRef.current !== null) {
            setDisplayMs(dragRef.current);
        } else {
            const { positionMs, wallClock, playing } = startRef.current;
            const elapsed = playing ? Date.now() - wallClock : 0;
            setDisplayMs(positionMs + elapsed);
        }
        rafRef.current = requestAnimationFrame(tick);
    }, []);

    useEffect(() => {
        rafRef.current = requestAnimationFrame(tick);
        return () => cancelAnimationFrame(rafRef.current);
    }, [tick]);

    // Called on every thumb move — freezes the display at the drag position.
    const onDragChange = useCallback((ms: number) => {
        dragRef.current = ms;
    }, []);

    // Called on pointer-up — seeks backend and restores normal extrapolation.
    const onDragCommit = useCallback(async (ms: number) => {
        dragRef.current = null;
        startRef.current = { positionMs: ms, wallClock: Date.now(), playing: startRef.current.playing };
        await invoke("seek", { toMs: ms });
    }, []);

    return { displayMs, durationMs, paused: isPaused, active: isAnyTrackActive, trackPath, trackTitle, trackArtist, onDragChange, onDragCommit, sync: poll };
}
