import { useEffect, useRef, useState } from "react";
import {
    FlatList,
    Pressable,
    StyleSheet,
    Text,
    TextInput,
    View,
} from "react-native";
import { StatusBar } from "expo-status-bar";
import { useDesktopSync } from "@/hooks/useDesktopSync";

function fmt(ms: number) {
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Extrapolates the current playback position from the last server snapshot. */
function livePosition(positionMs: number, playing: boolean, snapshotAtMs: number) {
    if (!playing) return positionMs;
    return positionMs + (Date.now() - snapshotAtMs);
}

export default function App() {
    const [urlInput, setUrlInput] = useState("");
    const [connectedUrl, setConnectedUrl] = useState<string | null>(null);
    const [query, setQuery] = useState("");
    const [displayMs, setDisplayMs] = useState(0);
    const rafRef = useRef<number>(0);

    const { status, playback, searchResults, search, play, pause, resume, stop, next, previous, seek } =
        useDesktopSync(connectedUrl);
    const [barWidth, setBarWidth] = useState(0);

    // Extrapolation loop
    useEffect(() => {
        const tick = () => {
            if (playback) {
                setDisplayMs(livePosition(playback.positionMs, playback.playing, playback.snapshotAtMs));
            }
            rafRef.current = requestAnimationFrame(tick);
        };
        rafRef.current = requestAnimationFrame(tick);
        return () => cancelAnimationFrame(rafRef.current);
    }, [playback]);

    // Search on query change (debounced)
    useEffect(() => {
        const t = setTimeout(() => search(query), 300);
        return () => clearTimeout(t);
    }, [query, search]);

    // ── Connect screen ──────────────────────────────────────────
    if (status !== "connected") {
        const isConnecting = status === "connecting";
        return (
            <View style={styles.center}>
                <StatusBar style="light" />
                <Text style={styles.title}>cadence</Text>
                <Text style={styles.subtitle}>Enter the WebSocket address shown in the desktop app</Text>
                <TextInput
                    style={styles.urlInput}
                    value={urlInput}
                    onChangeText={setUrlInput}
                    autoCapitalize="none"
                    autoCorrect={false}
                    keyboardType="url"
                    placeholder="ws://192.168.x.x:7878"
                    placeholderTextColor="#555"
                    editable={!isConnecting}
                />
                {status === "error" && (
                    <Text style={styles.error}>Could not connect. Check the address and try again.</Text>
                )}
                {isConnecting ? (
                    <View style={styles.connectingRow}>
                        <Text style={styles.connectingText}>Connecting…</Text>
                        <Pressable
                            style={styles.cancelBtn}
                            onPress={() => setConnectedUrl(null)}
                        >
                            <Text style={styles.cancelBtnText}>Cancel</Text>
                        </Pressable>
                    </View>
                ) : (
                    <Pressable
                        style={styles.connectBtn}
                        onPress={() => setConnectedUrl(urlInput.trim())}
                    >
                        <Text style={styles.connectBtnText}>Connect</Text>
                    </Pressable>
                )}
            </View>
        );
    }

    // ── Main screen ─────────────────────────────────────────────
    return (
        <View style={styles.main}>
            <StatusBar style="light" />

            {/* Header */}
            <View style={styles.header}>
                <Text style={styles.title}>cadence</Text>
                <Pressable onPress={() => { setConnectedUrl(null); setQuery(""); }}>
                    <Text style={styles.disconnect}>Disconnect</Text>
                </Pressable>
            </View>

            {/* Search */}
            <TextInput
                style={styles.searchInput}
                value={query}
                onChangeText={setQuery}
                placeholder="Search tracks…"
                placeholderTextColor="#555"
                autoCapitalize="none"
                autoCorrect={false}
            />

            {/* Results */}
            <FlatList
                data={searchResults}
                keyExtractor={(item) => String(item.id)}
                style={styles.list}
                renderItem={({ item }) => (
                    <Pressable style={styles.trackRow} onPress={() => play(item.path)}>
                        <Text style={styles.trackTitle} numberOfLines={1}>{item.title}</Text>
                        <Text style={styles.trackArtist} numberOfLines={1}>{item.artist}</Text>
                    </Pressable>
                )}
                ListEmptyComponent={
                    query.trim() ? <Text style={styles.empty}>No results</Text> : null
                }
            />

            {/* Now playing bar */}
            {playback && (
                <View style={styles.nowPlaying}>
                    <View style={styles.nowPlayingInfo}>
                        <Text style={styles.nowTitle} numberOfLines={1}>
                            {playback.title ?? playback.trackPath.split(/[\\/]/).at(-1)}
                        </Text>
                        <Text style={styles.nowArtist} numberOfLines={1}>
                            {playback.artist ?? "Unknown Artist"}
                        </Text>
                    </View>

                    <View style={styles.progressRow}>
                        <Text style={styles.time}>{fmt(displayMs)}</Text>
                        <Pressable
                            style={styles.progressBar}
                            hitSlop={{ top: 16, bottom: 16 }}
                            onLayout={(e) => setBarWidth(e.nativeEvent.layout.width)}
                            onPress={(e) => {
                                if (barWidth > 0 && playback.durationMs) {
                                    const toMs = Math.round((e.nativeEvent.locationX / barWidth) * playback.durationMs);
                                    setDisplayMs(toMs);
                                    seek(toMs);
                                }
                            }}
                        >
                            <View
                                style={[
                                    styles.progressFill,
                                    { width: `${Math.min(100, (displayMs / (playback.durationMs || 1)) * 100)}%` },
                                ]}
                            />
                        </Pressable>
                        <Text style={styles.time}>{fmt(playback.durationMs)}</Text>
                    </View>

                    <View style={styles.controls}>
                        <Pressable style={styles.ctrlBtn} onPress={previous}>
                            <Text style={styles.ctrlText}>Prev</Text>
                        </Pressable>
                        <Pressable style={styles.ctrlBtn} onPress={playback.playing ? pause : resume}>
                            <Text style={styles.ctrlText}>{playback.playing ? "Pause" : "Resume"}</Text>
                        </Pressable>
                        <Pressable style={styles.ctrlBtn} onPress={next}>
                            <Text style={styles.ctrlText}>Next</Text>
                        </Pressable>
                    </View>
                    <View style={[styles.controls, {marginTop: 8}]}>
                        <Pressable style={styles.ctrlBtn} onPress={stop}>
                            <Text style={styles.ctrlText}>Stop</Text>
                        </Pressable>
                    </View>
                </View>
            )}
        </View>
    );
}

const C = {
    bg: "#0a0a0a",
    surface: "#141414",
    border: "#222",
    accent: "#fff",
    muted: "#888",
    dim: "#555",
};

const styles = StyleSheet.create({
    center: {
        flex: 1,
        backgroundColor: C.bg,
        alignItems: "center",
        justifyContent: "center",
        padding: 24,
    },
    main: {
        flex: 1,
        backgroundColor: C.bg,
        paddingTop: 56,
    },
    header: {
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        paddingHorizontal: 16,
        marginBottom: 12,
    },
    title: {
        color: C.accent,
        fontSize: 22,
        fontWeight: "700",
    },
    subtitle: {
        color: C.muted,
        fontSize: 14,
        textAlign: "center",
        marginTop: 8,
        marginBottom: 24,
    },
    disconnect: {
        color: C.muted,
        fontSize: 13,
    },
    urlInput: {
        width: "100%",
        borderWidth: 1,
        borderColor: C.border,
        borderRadius: 8,
        padding: 12,
        color: C.accent,
        fontSize: 15,
        marginBottom: 8,
    },
    error: {
        color: "#e05555",
        fontSize: 13,
        marginBottom: 8,
        textAlign: "center",
    },
    connectingRow: {
        marginTop: 8,
        flexDirection: "row",
        alignItems: "center",
        gap: 16,
    },
    connectingText: {
        color: C.muted,
        fontSize: 15,
    },
    cancelBtn: {
        borderWidth: 1,
        borderColor: C.border,
        borderRadius: 8,
        paddingVertical: 8,
        paddingHorizontal: 16,
    },
    cancelBtnText: {
        color: C.accent,
        fontSize: 14,
    },
    connectBtn: {
        marginTop: 8,
        backgroundColor: C.surface,
        borderWidth: 1,
        borderColor: C.border,
        borderRadius: 8,
        paddingVertical: 12,
        paddingHorizontal: 32,
    },
    connectBtnText: {
        color: C.accent,
        fontSize: 15,
        fontWeight: "600",
    },
    searchInput: {
        marginHorizontal: 16,
        borderWidth: 1,
        borderColor: C.border,
        borderRadius: 8,
        padding: 10,
        color: C.accent,
        fontSize: 15,
        marginBottom: 8,
    },
    list: {
        flex: 1,
        paddingHorizontal: 16,
    },
    trackRow: {
        paddingVertical: 10,
        borderBottomWidth: 1,
        borderBottomColor: C.border,
    },
    trackTitle: {
        color: C.accent,
        fontSize: 15,
    },
    trackArtist: {
        color: C.muted,
        fontSize: 13,
        marginTop: 2,
    },
    empty: {
        color: C.dim,
        textAlign: "center",
        marginTop: 32,
        fontSize: 14,
    },
    nowPlaying: {
        borderTopWidth: 1,
        borderTopColor: C.border,
        backgroundColor: C.surface,
        padding: 16,
        paddingBottom: 28,
    },
    nowPlayingInfo: {
        marginBottom: 8,
    },
    nowTitle: {
        color: C.accent,
        fontSize: 15,
        fontWeight: "600",
    },
    nowArtist: {
        color: C.muted,
        fontSize: 13,
        marginTop: 2,
    },
    progressRow: {
        flexDirection: "row",
        alignItems: "center",
        gap: 8,
        marginBottom: 12,
    },
    time: {
        color: C.muted,
        fontSize: 12,
        width: 36,
    },
    progressBar: {
        flex: 1,
        height: 3,
        backgroundColor: C.border,
        borderRadius: 2,
        overflow: "hidden",
    },
    progressFill: {
        height: 3,
        backgroundColor: C.accent,
        borderRadius: 2,
    },
    controls: {
        flexDirection: "row",
        justifyContent: "center",
        gap: 12,
    },
    ctrlBtn: {
        borderWidth: 1,
        borderColor: C.border,
        borderRadius: 8,
        paddingVertical: 8,
        paddingHorizontal: 20,
    },
    ctrlText: {
        color: C.accent,
        fontSize: 14,
    },
});
