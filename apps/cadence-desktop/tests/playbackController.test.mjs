import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import ts from "typescript";

// Use the project's existing compiler; no browser or extra test dependency needed.
const source = await readFile(new URL("../src/hooks/playbackController.ts", import.meta.url), "utf8");
const { outputText } = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
});
const { PlaybackController, playbackClock, playbackPosition } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString("base64")}`);
const deferred = () => {
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    return { promise, resolve, reject };
};
const flush = () => new Promise(resolve => setImmediate(resolve));
const snapshot = (position_ms, path = "a.flac", paused = false) => ({
    path, position_ms, duration_ms: 100000, paused, title: null, artist: null, mode: "Default",
    output_state: "ready", playback_error: null,
});

test("recovery snapshots freeze the clock and ready snapshots restart it", () => {
    for (const now of [0, 1000, 2000]) {
        const clock = playbackClock({ ...snapshot(30000), output_state: "recovering" }, now);
        assert.equal(playbackPosition(clock, now + 999), 30000);
    }
    const recovered = playbackClock(snapshot(30000), 3000);
    assert.equal(playbackPosition(recovered, 3500), 30500);
    const paused = playbackClock(snapshot(30000, "a.flac", true), 3000);
    assert.equal(playbackPosition(paused, 3500), 30000);
    const failed = playbackClock({ ...snapshot(30000), output_state: "ready", playback_error: "Cannot restore" }, 3000);
    assert.equal(playbackPosition(failed, 3500), 30000);
});
async function harness() {
    const reads = [], seeks = [], applied = [], errors = [];
    const controller = new PlaybackController(
        () => { const d = deferred(); reads.push(d); return d.promise; },
        ms => { const d = deferred(); seeks.push({ ...d, ms }); return d.promise; },
        (status, changed) => applied.push({ status, changed, preview: controller.preview }),
        error => errors.push(error),
    );
    const initial = controller.poll();
    reads[0].resolve(snapshot(0));
    await initial;
    return { controller, reads, seeks, applied, errors };
}

test("holds preview, discards pre-seek polls, and sends only the latest pending target", async () => {
    const { controller: c, reads, seeks, applied } = await harness();
    const stale = c.poll();
    c.seek(10000); c.seek(20000); c.seek(30000);
    assert.deepEqual(seeks.map(s => s.ms), [10000]);
    reads[1].resolve(snapshot(100)); await stale;
    assert.equal(applied.length, 1);
    assert.equal(c.preview, 30000);
    seeks[0].resolve(); await flush();
    reads[2].resolve(snapshot(10000)); await flush();
    assert.deepEqual(seeks.map(s => s.ms), [10000, 30000]);
    assert.equal(c.preview, 30000);
    seeks[1].resolve(); await flush();
    reads[3].resolve(snapshot(30010)); await flush();
    assert.equal(c.preview, null);
    assert.equal(applied.at(-1).status.position_ms, 30010);
});

test("a newer poll can settle a seek without leaving the cursor stuck", async () => {
    const { controller: c, reads, seeks, applied } = await harness();
    c.seek(10000); seeks[0].resolve(); await flush();
    const newer = c.poll();
    reads[2].resolve(snapshot(10020)); await newer;
    reads[1].resolve(snapshot(10000)); await flush();
    assert.equal(c.preview, null);
    assert.equal(applied.at(-1).status.position_ms, 10020);
});

test("polling during a replacement seek keeps its preview until completion", async () => {
    const { controller: c, reads, seeks } = await harness();
    c.seek(10000); seeks[0].resolve(); await flush();
    c.seek(20000);
    reads[1].resolve(snapshot(10000)); await flush();
    assert.equal(seeks.length, 2);
    const duringSeek = c.poll();
    reads[2].resolve(snapshot(10000)); await duringSeek;
    assert.equal(c.preview, 20000);
    seeks[1].resolve(); await flush();
    reads[3].resolve(snapshot(20000)); await flush();
    assert.equal(c.preview, null);
});

test("failed paused seek reports an error and restores authoritative paused position", async () => {
    const { controller: c, reads, seeks, applied, errors } = await harness();
    c.seek(20000); seeks[0].reject(new Error("bad seek")); await flush();
    reads[1].resolve(snapshot(1234, "a.flac", true)); await flush();
    assert.match(errors.at(-1), /bad seek/);
    assert.equal(c.preview, null);
    assert.equal(applied.at(-1).status.position_ms, 1234);
    assert.equal(applied.at(-1).status.paused, true);
});

test("remote track changes drop pending seeks for the previous track", async () => {
    const { controller: c, reads, seeks, applied } = await harness();
    c.seek(10000); c.seek(20000);
    seeks[0].resolve(); await flush();
    reads[1].resolve(snapshot(0, "b.flac")); await flush();
    assert.equal(seeks.length, 1);
    assert.equal(c.preview, null);
    assert.equal(applied.at(-1).changed, true);
});

test("local stop/replay invalidates in-flight results even for the same path", async () => {
    const { controller: c, reads, seeks, applied } = await harness();
    c.seek(10000); c.seek(20000);
    const stale = c.poll(); c.cancel();
    const fresh = c.poll(); reads[2].resolve(snapshot(0)); await fresh;
    reads[1].resolve(snapshot(10000)); await stale;
    seeks[0].resolve(); await flush();
    assert.equal(seeks.length, 1);
    assert.equal(applied.at(-1).status.position_ms, 0);
    c.seek(5000);
    assert.equal(seeks.at(-1).ms, 5000);
});

test("failed status refresh cancels queued seeks rather than guessing the active track", async () => {
    const { controller: c, reads, seeks, errors } = await harness();
    c.seek(10000); c.seek(20000); seeks[0].resolve(); await flush();
    reads[1].reject(new Error("disconnected")); await flush();
    assert.equal(c.preview, null);
    assert.equal(seeks.length, 1);
    assert.match(errors.at(-1), /disconnected/);
});
