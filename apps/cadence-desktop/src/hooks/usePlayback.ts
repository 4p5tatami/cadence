import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PlaybackController, playbackClock, playbackPosition, type PlaybackStatus } from "./playbackController";

export function usePlayback() {
    const startRef = useRef({ positionMs: 0, wallClock: 0, playing: false, durationMs: 0 });
    const dragRef = useRef<number | null>(null);
    const [displayMs, setDisplayMs] = useState(0);
    const [status, setStatus] = useState<PlaybackStatus | null>(null);
    const [seekError, setSeekError] = useState<string | null>(null);
    const controllerRef = useRef<PlaybackController | null>(null);
    if (controllerRef.current === null) {
        controllerRef.current = new PlaybackController(
            () => invoke<PlaybackStatus | null>("status"),
            (toMs) => invoke("seek", { toMs }),
            (snapshot, changedTrack) => {
                if (changedTrack) {
                    dragRef.current = null;
                }
                setStatus(snapshot);
                if (dragRef.current === null && controllerRef.current?.preview === null) {
                    startRef.current = playbackClock(snapshot, performance.now());
                }
            },
            setSeekError,
        );
    }
    const controller = controllerRef.current;

    useEffect(() => {
        void controller.poll();
        const interval = setInterval(() => { void controller.poll(); }, 1000);
        return () => {
            clearInterval(interval);
            controller.cancel();
        };
    }, [controller]);

    useEffect(() => {
        let frame: number;
        const tick = () => {
            const { durationMs } = startRef.current;
            const position = dragRef.current ?? controller.preview ??
                playbackPosition(startRef.current, performance.now());
            setDisplayMs(Math.max(0, Math.min(position, durationMs)));
            frame = requestAnimationFrame(tick);
        };
        frame = requestAnimationFrame(tick);
        return () => cancelAnimationFrame(frame);
    }, [controller]);

    const onDragChange = useCallback((ms: number) => { dragRef.current = ms; }, []);
    const onDragCommit = useCallback((ms: number) => {
        dragRef.current = null;
        controller.seek(Math.max(0, Math.min(ms, startRef.current.durationMs)));
    }, [controller]);
    const cancelSeek = useCallback(() => {
        dragRef.current = null;
        setSeekError(null);
        controller.cancel();
    }, [controller]);

    return {
        displayMs, durationMs: status?.duration_ms ?? 0, paused: status?.paused ?? true,
        active: status !== null, trackPath: status?.path ?? null,
        trackTitle: status?.title ?? null, trackArtist: status?.artist ?? null,
        mode: status?.mode ?? "Default", onDragChange, onDragCommit,
        outputState: status?.output_state ?? "ready", playbackError: status?.playback_error ?? null,
        sync: controller.poll, cancelSeek, seekError, clearSeekError: () => setSeekError(null),
    };
}
