export interface PlaybackStatus {
    path: string;
    duration_ms: number;
    position_ms: number;
    paused: boolean;
    output_state: "ready" | "recovering";
    playback_error: string | null;
    title: string | null;
    artist: string | null;
    mode: "Default" | "Shuffle" | "Replay";
}

export function playbackClock(snapshot: PlaybackStatus | null, now: number) {
    return {
        positionMs: snapshot?.position_ms ?? 0,
        wallClock: now,
        playing: snapshot !== null && !snapshot.paused && snapshot.output_state === "ready" && !snapshot.playback_error,
        durationMs: snapshot?.duration_ms ?? 0,
    };
}

export function playbackPosition(clock: ReturnType<typeof playbackClock>, now: number) {
    return Math.max(0, Math.min(clock.positionMs + (clock.playing ? now - clock.wallClock : 0), clock.durationMs));
}

/** Serializes seeks and prevents pre-seek/out-of-order snapshots moving the cursor. */
export class PlaybackController {
    preview: number | null = null;
    private pending: number | null = null;
    private busy = false;
    private awaitingStatus = false;
    private generation = 0;
    private revision = 0;
    private request = 0;
    private applied = 0;
    private path: string | null = null;

    constructor(
        private readStatus: () => Promise<PlaybackStatus | null>,
        private sendSeek: (ms: number) => Promise<unknown>,
        private applyStatus: (status: PlaybackStatus | null, changedTrack: boolean) => void,
        private reportError: (error: string | null) => void,
    ) {}

    cancel = () => {
        this.generation++;
        this.revision++;
        this.pending = this.preview = null;
        this.awaitingStatus = false;
        this.path = null;
    };

    poll = async (settleSeek = false) => {
        const revision = this.revision;
        const request = ++this.request;
        try {
            const status = await this.readStatus();
            if (revision !== this.revision || request < this.applied) return;
            this.applied = request;
            const changedTrack = (status?.path ?? null) !== this.path;
            if (changedTrack) {
                this.cancel();
                this.path = status?.path ?? null;
            } else if (this.awaitingStatus && this.pending === null) {
                this.preview = null;
                this.awaitingStatus = false;
            }
            this.applyStatus(status, changedTrack);
        } catch (error) {
            if (revision !== this.revision || request < this.applied) return;
            // Do not send another queued seek when the active track is unknown.
            if (settleSeek) this.cancel();
            this.reportError(`Playback sync failed: ${String(error)}`);
        }
    };

    seek = (ms: number) => {
        if (this.path === null) return;
        this.revision++;
        this.pending = this.preview = ms;
        this.reportError(null);
        if (!this.busy) void this.drain();
    };

    private async drain() {
        this.busy = true;
        try {
            while (this.pending !== null) {
                const target = this.pending;
                const generation = this.generation;
                this.pending = null;
                this.awaitingStatus = false;
                try {
                    await this.sendSeek(target);
                } catch (error) {
                    if (generation === this.generation) {
                        this.reportError(`Seek failed: ${String(error)}`);
                    }
                }
                if (generation === this.generation) {
                    this.revision++;
                    this.awaitingStatus = true;
                    // Verify track identity before sending the newest queued seek.
                    await this.poll(true);
                }
            }
        } finally {
            this.busy = false;
        }
    }
}
