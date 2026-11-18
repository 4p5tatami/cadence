import { Pressable, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { router } from "expo-router";
import { colors, radius, spacing } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";
import { usePlaybackPosition } from "@/hooks/usePlaybackPosition";

/**
 * Bottom overlay shown above the tab bar whenever something is playing.
 * Tapping it opens the Now Playing screen.
 */
export function MiniPlayer() {
    const { playback, pause, resume } = useCadence();
    const positionMs = usePlaybackPosition(playback, 500);
    if (!playback) return null;

    const progress = playback.durationMs ? Math.min(1, positionMs / playback.durationMs) : 0;
    const title = playback.title ?? playback.trackPath.split(/[\\/]/).at(-1) ?? "Unknown";

    return (
        <View style={styles.wrap}>
            <View style={styles.track}>
                <View style={[styles.fill, { width: `${progress * 100}%` }]} />
            </View>

            <Pressable style={styles.card} onPress={() => router.navigate("/now-playing")}>
                {/* TODO: album artwork once the desktop sends it */}
                <View style={styles.art}>
                    <Feather name="music" size={20} color={colors.textDim} />
                </View>

                <View style={styles.meta}>
                    <Text style={styles.title} numberOfLines={1}>{title}</Text>
                    <Text style={styles.artist} numberOfLines={1}>{playback.artist ?? "Unknown Artist"}</Text>
                </View>

                <Pressable
                    style={styles.playBtn}
                    onPress={playback.playing ? pause : resume}
                    hitSlop={8}
                    accessibilityRole="button"
                    accessibilityLabel={playback.playing ? "Pause" : "Play"}
                >
                    <Feather name={playback.playing ? "pause" : "play"} size={20} color={colors.text} />
                </Pressable>
            </Pressable>
        </View>
    );
}

const styles = StyleSheet.create({
    wrap: { paddingHorizontal: spacing.md, paddingBottom: spacing.sm, backgroundColor: colors.bg },
    track: {
        height: 3,
        marginHorizontal: spacing.sm,
        marginBottom: spacing.md,
        borderRadius: 2,
        backgroundColor: colors.divider,
        overflow: "hidden",
    },
    fill: { height: 3, backgroundColor: colors.text },
    card: {
        flexDirection: "row",
        alignItems: "center",
        padding: spacing.md,
        gap: spacing.lg,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface,
    },
    art: {
        width: 52,
        height: 52,
        borderRadius: radius.sm,
        backgroundColor: colors.surfaceRaised,
        alignItems: "center",
        justifyContent: "center",
    },
    meta: { flex: 1 },
    title: { fontSize: 16, fontWeight: "500", color: colors.text },
    artist: { fontSize: 14, color: colors.textMuted, marginTop: 2 },
    playBtn: {
        width: 44,
        height: 44,
        borderRadius: radius.pill,
        backgroundColor: colors.bg,
        alignItems: "center",
        justifyContent: "center",
    },
});
