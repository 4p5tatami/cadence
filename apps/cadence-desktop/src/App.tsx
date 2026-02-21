import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { usePlayback } from "@/hooks/usePlayback";

function fmt(ms: number) {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function App() {
    const { displayMs, durationMs, paused, active, trackPath, onDragChange, onDragCommit, sync } = usePlayback();

    const handleBrowse = async () => {
        const path = await open({
            multiple: false,
            filters: [{ name: "Audio", extensions: ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "wma"] }],
        });
        if (!path) return;
        await invoke("play", { path });
        await sync();
    };

    const handlePause = async () => {
        await invoke(paused ? "resume" : "pause");
        await sync();
    };

    const handleStop = async () => {
        await invoke("stop");
        await sync();
    };

    const handleAdvance = async (seconds: number) => {
        await invoke("advance", { deltaMs: seconds * 1000 });
    };

    const filename = trackPath ? (trackPath.split("\\").at(-1) ?? trackPath) : null;

    return (
        <main style={{ fontFamily: "Roboto", padding: "1rem", fontWeight: 500 }}>
            <h2>cadence</h2>
            <Button variant={"outline"} onClick={handleBrowse} style={{ marginTop: "1.5rem" }}>
                Browse
            </Button>
            {filename && (
                <p style={{ color: "#888", marginTop: "1.5rem" }}>{filename}</p>
            )}
            {active && (
                <div style={{ marginTop: "1.5rem" }}>
                    <Slider
                        value={[displayMs]}
                        min={0}
                        max={durationMs || 1}
                        step={500}
                        onValueChange={([ms]) => onDragChange(ms)}
                        onValueCommit={([ms]) => { onDragCommit(ms); }}
                    />
                    <div style={{ display: "flex", justifyContent: "space-between", marginTop: "0.25rem", color: "#888", fontSize: "0.75rem" }}>
                        <span>{fmt(displayMs)}</span>
                        <span>{fmt(durationMs)}</span>
                    </div>
                </div>
            )}
            {active && (
                <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", marginTop: "1rem" }}>
                    <Button variant={"outline"} onClick={() => handleAdvance(-10)}>-10s</Button>
                    <Button variant={"outline"} onClick={handlePause}>{paused ? "Resume" : "Pause"}</Button>
                    <Button variant={"outline"} onClick={() => handleAdvance(10)}>+10s</Button>
                </div>
            )}
            {active && (
                <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", marginTop: "1rem" }}>
                    <Button variant={"outline"} onClick={handleStop}>Stop</Button>
                </div>
            )}
        </main>
    );
}

export default App;
