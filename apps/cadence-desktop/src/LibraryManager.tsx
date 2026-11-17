import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

interface LibraryRecord {
    id: number;
    path: string;
}

interface IndexProgress {
    path: string;
    scanned: number;
}

interface Props {
    onBack: () => void;
}

export function LibraryManager({ onBack }: Props) {
    const [libraries, setLibraries] = useState<LibraryRecord[]>([]);
    const [selectedId, setSelectedId] = useState<number | null>(null);
    const [status, setStatus] = useState<string | null>(null);
    const [indexing, setIndexing] = useState(false);

    const refresh = () => {
        invoke<LibraryRecord[]>("list_libraries").then(setLibraries);
    };

    useEffect(() => { refresh(); }, []);

    useEffect(() => {
        const unlisten = listen<IndexProgress>("index-progress", ({ payload }) => {
            setStatus(`Indexing… ${payload.scanned.toLocaleString()} tracks scanned`);
        });
        return () => { void unlisten.then(fn => fn()); };
    }, []);

    const handleAdd = async () => {
        const dir = await open({ directory: true, multiple: false });
        if (!dir) return;
        setIndexing(true);
        setStatus("Indexing…");
        try {
            const count = await invoke<number>("index_library", { path: dir });
            setStatus(`${count.toLocaleString()} tracks indexed`);
        } catch (error) {
            setStatus(`Indexing failed: ${String(error)}`);
        } finally {
            setIndexing(false);
            refresh();
        }
    };

    const handleRemove = async () => {
        if (selectedId === null) return;
        try {
            await invoke("delete_library", { id: selectedId });
            setStatus(null);
        } catch (error) {
            setStatus(`Remove failed: ${String(error)}`);
        }
        setSelectedId(null);
        refresh();
    };

    return (
        <div style={{ fontFamily: "Roboto", padding: "1rem", fontWeight: 500 }}>
            <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "1.5rem" }}>
                <button
                    onClick={onBack}
                    style={{ background: "none", border: "none", color: "inherit", cursor: "pointer", fontSize: "1rem", padding: 0 }}
                >←</button>
                <h2 style={{ margin: 0 }}>Libraries</h2>
            </div>

            <Button variant="outline" disabled={indexing} onClick={() => void handleAdd()}>Add Folder</Button>

            {status && (
                <p style={{ color: "#888", marginTop: "0.5rem", fontSize: "0.8rem" }}>{status}</p>
            )}

            <ul style={{ listStyle: "none", padding: 0, marginTop: "1rem", flex: 1 }}>
                {libraries.length === 0 && (
                    <li style={{ color: "#555", fontSize: "0.9rem" }}>No libraries added yet.</li>
                )}
                {libraries.map(lib => (
                    <li
                        key={lib.id}
                        onClick={() => setSelectedId(lib.id === selectedId ? null : lib.id)}
                        style={{
                            padding: "0.4rem 0.6rem",
                            borderRadius: "4px",
                            cursor: "pointer",
                            fontSize: "0.9rem",
                            borderBottom: "1px solid #222",
                            background: lib.id === selectedId ? "#2a2a2a" : "transparent",
                            wordBreak: "break-all",
                        }}
                    >
                        {lib.path}
                    </li>
                ))}
            </ul>

            <div style={{ marginTop: "1rem" }}>
                <Button variant="outline" disabled={selectedId === null || indexing} onClick={() => void handleRemove()}>Remove</Button>
            </div>
        </div>
    );
}
