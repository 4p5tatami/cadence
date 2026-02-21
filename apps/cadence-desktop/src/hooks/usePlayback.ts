import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface StatusResponse {
    path: string | null;
    duration_ms: number | null;
    position_ms: number;
    paused: boolean;
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
    const [isDragging, setIsDragging] = useState(false);
    const [trackPath, setTrackPath] = useState<string | null>(null);

    // Polls backend and resets the extrapolation anchor.
    const poll = useCallback(async () => {
        const status = await invoke<StatusResponse>("status");
        // Don't clobber the anchor while the user is scrubbing.
        if (dragRef.current === null) {
            startRef.current = {
                positionMs: status.position_ms,
                wallClock: Date.now(),
                playing: !status.paused && status.path !== null,
            };
        }
        setIsPaused(status.paused || status.path === null);
        setIsAnyTrackActive(status.path !== null);
        setTrackPath(status.path);
        setDurationMs(status.duration_ms ?? 0);
    }, []);

    // Poll every second.
    useEffect(() => {
        poll();
        const id = setInterval(poll, 1000);
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
        setIsDragging(true);
    }, []);

    // Called on pointer-up — seeks backend and restores normal extrapolation.
    const onDragCommit = useCallback(async (ms: number) => {
        dragRef.current = null;
        setIsDragging(false);
        startRef.current = { positionMs: ms, wallClock: Date.now(), playing: startRef.current.playing };
        await invoke("seek", { toMs: ms });
    }, []);

    return { displayMs, durationMs, paused: isPaused, active: isAnyTrackActive, isDragging, trackPath, onDragChange, onDragCommit, sync: poll };
}
