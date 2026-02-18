import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

function App() {
  const [nowPlaying, setNowPlaying] = useState<string | null>(null);
  const [paused, setPaused] = useState(false)

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
  }

  const handleStop = async () => {
    await invoke("stop");
    setNowPlaying(null);
    setPaused(true);
  }

  const handleAdvance = async (seconds: number) => {
    await invoke("advance", { deltaMs: seconds * 1000 });
  }

  return (
    <main style={{ fontFamily: "monospace", padding: "1rem" }}>
      <h2>cadence</h2>
      <button onClick={handleBrowse}>Browse</button>
      {nowPlaying && (
        <p style={{ color: "#888", marginTop: "1rem" }}>
          Now playing: {(nowPlaying.split("\\").at(-1) ?? nowPlaying)}
        </p>
      )}
      {nowPlaying && (
          <button onClick={handlePause}>{paused ? "Resume" : "Pause"}</button>)}
      {nowPlaying && <button onClick={() => handleAdvance(-10)}>-10s</button>}
      {nowPlaying && <button onClick={() => handleAdvance(10)}>+10s</button>}
      {nowPlaying && (
          <button onClick={handleStop}>{"Stop"}</button>)}
    </main>
  );
}

export default App;
