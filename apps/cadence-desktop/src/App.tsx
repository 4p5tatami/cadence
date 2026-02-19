import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { Button } from "@/components/ui/button";

function App() {
    const [nowPlaying, setNowPlaying] = useState<string | null>(null);
    const [paused, setPaused] = useState(false);

    const handleBrowse = async () => {
        const path = await open({
            multiple: false,
            filters: [{ name: "Audio", extensions: ["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "wma"] }],
        });
        if (!path) return;
        await invoke("play", { path });
        setNowPlaying(path);
        setPaused(false);
    };

    const handlePause = async () => {
        if (paused) {
            await invoke("resume");
        } else {
            await invoke("pause");
        }
        setPaused(!paused);
    };

    const handleStop = async () => {
        await invoke("stop");
        setNowPlaying(null);
        setPaused(true);
    };

    const handleAdvance = async (seconds: number) => {
        await invoke("advance", { deltaMs: seconds * 1000 });
    };

    return (
        <main style={{ fontFamily: "Roboto", padding: "1rem", fontWeight: 500 }}>
            <h2>cadence</h2>
            <Button variant={"outline"} onClick={handleBrowse} style={{ marginTop: "1.5rem" }}>
                Browse
            </Button>
            {nowPlaying && (
                <p style={{ color: "#888", marginTop: "1.5rem" }}>
                    Now playing: {(nowPlaying.split("\\").at(-1) ?? nowPlaying)}
                </p>
            )}
            {nowPlaying && (
                <Button variant={"outline"} onClick={handlePause} style={{ marginTop: "1.5rem" }}>
                    {paused ? "Resume" : "Pause"}
                </Button>
            )}
            {nowPlaying && (
                <Button variant={"outline"} onClick={() => handleAdvance(-10)} style={{ marginLeft: "0.5rem" }}>
                    -10s
                </Button>
            )}
            {nowPlaying && (
                <Button variant={"outline"} onClick={() => handleAdvance(10)} style={{ marginLeft: "0.5rem" }}>
                    +10s
                </Button>
            )}
            {nowPlaying && (
                <Button variant={"outline"} onClick={handleStop} style={{ marginLeft: "0.5rem" }}>
                    Stop
                </Button>
            )}
        </main>
    );
}

export default App;
