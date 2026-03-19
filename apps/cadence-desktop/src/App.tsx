import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import { usePlayback } from "@/hooks/usePlayback";
import { LibraryManager } from "@/LibraryManager";
import { useEffect, useRef, useState } from "react";

interface TrackRecord {
    id: number;
    path: string;
    title: string;
    artist: string;
    duration_ms: number;
}

function fmt(ms: number) {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function App() {
    const { displayMs, durationMs, paused, active, trackPath, trackTitle, trackArtist, volume, onDragChange, onDragCommit, sync } = usePlayback();

    const [view, setView] = useState<"main" | "libraries">("main");
    const [query, setQuery] = useState("");
    const [results, setResults] = useState<TrackRecord[]>([]);
    const [wsAddr, setWsAddr] = useState<string | null>(null);
    const [menuOpen, setMenuOpen] = useState(false);
    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!menuOpen) return;
        const handler = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                setMenuOpen(false);
            }
        };
        document.addEventListener("mousedown", handler);
        return () => document.removeEventListener("mousedown", handler);
    }, [menuOpen]);

    useEffect(() => {
        invoke<string>("ws_address").then(setWsAddr);
    }, []);

    useEffect(() => {
        if (!query.trim()) { setResults([]); return; }
        invoke<TrackRecord[]>("search_tracks", { query }).then(setResults);
    }, [query]);

    const handlePlayResult = async (path: string) => {
        await invoke("play", { path });
        await sync();
    };

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

    const handlePrevious = async () => {
        await invoke("previous");
    };

    const handleNext = async () => {
        await invoke("next");
    }

    // const handleSetVolume = async (vol: number) => {
    //     await invoke("set_volume", { vol: vol });
    // }
    //
    // const handleStepVolume = async (delta: number) => {
    //     const target_volume = Math.min(1.0, Math.max(0.0, volume + delta));
    //     await invoke("set_volume", { target: target_volume });
    // };

    if (view === "libraries") {
        return <LibraryManager onBack={() => setView("main")} />;
    }

    return (
        <main style={{ fontFamily: "Roboto", padding: "1rem", fontWeight: 500, position: "relative" }}>
            <h2>cadence</h2>

            <div ref={menuRef} style={{ position: "absolute", top: "1rem", right: "1rem" }}>
                <Button variant="outline" onClick={() => setMenuOpen(o => !o)}>☰</Button>
                {menuOpen && (
                    <div style={{ position: "absolute", right: 0, top: "calc(100% + 4px)", background: "#1a1a1a", border: "1px solid #333", borderRadius: "6px", minWidth: "140px", zIndex: 100, overflow: "hidden" }}>
                        <button
                            onClick={() => { setMenuOpen(false); setView("libraries"); }}
                            style={{ display: "block", width: "100%", padding: "0.5rem 0.75rem", background: "none", border: "none", color: "inherit", cursor: "pointer", textAlign: "left", fontSize: "0.9rem" }}
                        >Index libraries</button>
                        <button
                            onClick={() => { setMenuOpen(false); void handleBrowse(); }}
                            style={{ display: "block", width: "100%", padding: "0.5rem 0.75rem", background: "none", border: "none", color: "inherit", cursor: "pointer", textAlign: "left", fontSize: "0.9rem" }}
                        >Play file</button>
                    </div>
                )}
            </div>

            <div style={{ display: "flex", gap: "0.5rem", marginTop: "1.5rem", alignItems: "center" }}>
                <input
                    value={query}
                    onChange={e => setQuery(e.target.value)}
                    placeholder="Search tracks…"
                    style={{ flex: 1, padding: "0.4rem 0.6rem", fontSize: "0.9rem", border: "1px solid #333", borderRadius: "4px", background: "transparent", color: "inherit", outline: "none" }}
                />
            </div>

            {!active && results.length == 0 && (
                <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.7rem" }}>
                    <Button variant={"outline"} onClick={() => handleNext()}>Play Something</Button>
                </div>
            )}

            {results.length > 0 && (
                <ul style={{ listStyle: "none", padding: 0, marginTop: "0.75rem", maxHeight: "40vh", overflowY: "auto" }}>
                    {results.map(r => (
                        <li
                            key={r.id}
                            onClick={() => void handlePlayResult(r.path)}
                            style={{ padding: "0.4rem 0.5rem", cursor: "pointer", borderRadius: "4px", borderBottom: "1px solid #222" }}
                        >
                            <span>{r.title}</span>
                            <span style={{ color: "#888", marginLeft: "0.5rem", fontSize: "0.85rem" }}>{r.artist}</span>
                        </li>
                    ))}
                </ul>
            )}

            {active && (
                <div style={{ marginTop: "1.5rem" }}>
                    <p style={{ margin: 0 }}>{trackTitle ?? trackPath?.split("\\").at(-1) ?? trackPath}</p>
                    <p style={{ margin: 0, color: "#888", fontSize: "0.85rem" }}>{trackArtist ?? "Unknown Artist"}</p>
                </div>
            )}

            {active && (
                <div style={{ marginTop: "1.5rem" }}>
                    <Slider
                        value={[displayMs]}
                        min={0}
                        max={durationMs || 1}
                        step={500}
                        onValueChange={([ms]) => onDragChange(ms)}
                        onValueCommit={([ms]) => { void onDragCommit(ms); }}
                    />
                    <div style={{ display: "flex", justifyContent: "space-between", marginTop: "0.25rem", color: "#888", fontSize: "0.75rem" }}>
                        <span>{fmt(displayMs)}</span>
                        <span>{fmt(durationMs)}</span>
                    </div>
                </div>
            )}

            {active && (
                <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", marginTop: "0.7rem" }}>
                    <Button variant={"outline"} onClick={() => handlePrevious()}>Prev</Button>
                    <Button variant={"outline"} onClick={handlePause}>{paused ? "Resume" : "Pause"}</Button>
                    <Button variant={"outline"} onClick={() => handleNext()}>Next</Button>
                </div>
            )}

            {active && (
                <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", marginTop: "0.7rem" }}>
                    <Button variant={"outline"} onClick={handleStop}>Stop</Button>
                </div>
            )}

            {wsAddr && <p style={{ position: "fixed", bottom: "1rem", left: "1rem", margin: 0, color: "#A1A8B3", fontSize: "0.85rem" }}>websocket listening on {wsAddr}</p>}

            <div style={{ position: "fixed", bottom: "1rem", right: "1rem", display: "flex", flexDirection: "column", alignItems: "center", gap: "0.4rem" }}>
                <span style={{ color: "#A1A8B3", fontSize: "0.75rem" }}>{Math.round(volume * 100)}%</span>
                <Slider
                    orientation="vertical"
                    value={[volume]}
                    min={0}
                    max={1}
                    step={0.01}
                    style={{ height: "80px" }}
                    onValueCommit={([v]) => void invoke("set_volume", { target: v })}
                    onValueChange={([v]) => void invoke("set_volume", { target: v })}
                />
                <span style={{ color: "#A1A8B3", fontSize: "0.75rem" }}>Volume</span>
            </div>

        </main>
    );
}

export default App;
