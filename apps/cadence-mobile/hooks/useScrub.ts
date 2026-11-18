import { useCallback, useEffect, useRef, useState } from "react";

interface ScrubOptions {
    /** How close (0..1) the server value must get to the released value to count as confirmed. */
    tolerance: number;
    /** Give up waiting for confirmation after this long and show the server value again. */
    timeoutMs?: number;
}

/**
 * Optimistic value for a slider driven by the desktop. While dragging, and after
 * release until the desktop's state broadcast catches up, `shown` is the local
 * value, so the thumb doesn't jump back to the stale server value.
 */
export function useScrub(value: number, { tolerance, timeoutMs = 1500 }: ScrubOptions) {
    const [scrub, setScrub] = useState<number | null>(null);
    const dragging = useRef(false);
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

    // Drop the local value once the desktop reports something close to it.
    useEffect(() => {
        if (scrub !== null && !dragging.current && Math.abs(value - scrub) <= tolerance) setScrub(null);
    }, [value, scrub, tolerance]);

    useEffect(() => () => clearTimeout(timer.current), []);

    const start = useCallback(() => {
        dragging.current = true;
        clearTimeout(timer.current);
    }, []);

    const move = useCallback((v: number) => setScrub(v), []);

    const end = useCallback(
        (v: number) => {
            dragging.current = false;
            setScrub(v);
            clearTimeout(timer.current);
            timer.current = setTimeout(() => setScrub(null), timeoutMs);
        },
        [timeoutMs],
    );

    return { shown: scrub ?? value, start, move, end };
}
