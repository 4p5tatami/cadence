import { Pressable, StyleSheet, Text, View } from "react-native";
import { Feather, Ionicons } from "@expo/vector-icons";
import { router } from "expo-router";
import { SeekBar } from "@/components/SeekBar";
import { colors, fonts, radius, spacing } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";

/**
 * Bottom overlay shown above the tab bar whenever something is playing.
 * The progress line is seekable; tapping the card opens Now Playing.
 */
export function MiniPlayer() {
    const { playback, pause, resume } = useCadence();
    if (!playback) return null;

    const title = playback.title ?? playback.trackPath.split(/[\\/]/).at(-1) ?? "Unknown";

    return (
        <View style={styles.wrap}>
            <SeekBar variant="mini" style={styles.seek} />

            <Pressable style={styles.card} onPress={() => router.navigate("/now-playing")}>
                {/* TODO: album artwork once the desktop sends it */}
                <View style={styles.art}>
                    <Feather name="music" size={20} color={colors.accentIcon} />
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
                    <Ionicons
                        name={playback.playing ? "pause" : "play"}
                        size={22}
                        color={colors.text}
                        style={!playback.playing && styles.playNudge}
                    />
                </Pressable>
            </Pressable>
        </View>
    );
}

const styles = StyleSheet.create({
    wrap: { paddingHorizontal: spacing.sm, paddingBottom: spacing.sm, backgroundColor: colors.bg },
    // The 22 pt touch area centres the 3 pt line, leaving ~13 pt to the card as in the mockup.
    seek: { marginHorizontal: 6, marginBottom: 2 },
    card: {
        flexDirection: "row",
        alignItems: "center",
        padding: spacing.md,
        gap: spacing.lg,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.surfaceBorder,
        backgroundColor: colors.surface,
    },
    art: {
        width: 46,
        height: 46,
        borderRadius: radius.sm,
        backgroundColor: colors.accentTile,
        alignItems: "center",
        justifyContent: "center",
    },
    meta: { flex: 1, gap: 2 },
    title: { fontFamily: fonts.medium, fontSize: 15, lineHeight: 20, color: colors.text },
    artist: { fontFamily: fonts.regular, fontSize: 13, lineHeight: 18, color: colors.textMuted },
    playBtn: {
        width: 42,
        height: 42,
        borderRadius: radius.pill,
        backgroundColor: colors.bg,
        alignItems: "center",
        justifyContent: "center",
    },
    // Optically centre the play triangle.
    playNudge: { marginLeft: 2 },
});
