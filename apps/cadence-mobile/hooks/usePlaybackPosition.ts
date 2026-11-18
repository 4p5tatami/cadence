import { useEffect, useState } from "react";
import type { PlaybackState } from "@/hooks/useDesktopSync";

/**
 * Extrapolates the live playback position from the last server snapshot.
 * Ticks at `intervalMs` while playing; returns the snapshot as-is when paused.
 */
export function usePlaybackPosition(playback: PlaybackState | null, intervalMs = 250) {
    const compute = () => {
        if (!playback) return 0;
        const pos = playback.playing
            ? playback.positionMs + (Date.now() - playback.snapshotAtMs)
            : playback.positionMs;
        return Math.min(pos, playback.durationMs || pos);
    };

    const [positionMs, setPositionMs] = useState(compute);

    useEffect(() => {
        setPositionMs(compute());
        if (!playback?.playing) return;
        const id = setInterval(() => setPositionMs(compute()), intervalMs);
        return () => clearInterval(id);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [playback, intervalMs]);

    return positionMs;
}

export function formatMs(ms: number) {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
