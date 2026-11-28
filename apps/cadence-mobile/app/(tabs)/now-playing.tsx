import { Pressable, StyleSheet, Text, useWindowDimensions, View } from "react-native";
import { Feather, Ionicons, MaterialCommunityIcons } from "@expo/vector-icons";
import { router } from "expo-router";
import { Screen } from "@/components/Screen";
import { SeekBar } from "@/components/SeekBar";
import { colors, fonts, GUTTER, radius, spacing, type } from "@/constants/theme";
import type { PlaybackState } from "@/hooks/useDesktopSync";
import { useCadence } from "@/providers/CadenceProvider";

// Layout measured from the Now Playing mockup (390 pt wide screen).
const ART_INSET = 34; // artwork edge to screen edge
const ART_TOP = 12;
const SEEK_INSET = 29; // seek bar edge to screen edge
const PLAY_SIZE = 66;

/** Now Playing. Shuffle/repeat, queue, like and the overflow menu are left out until the desktop supports them. */
export default function NowPlayingScreen() {
    const { playback, device, status, pause, resume, next, previous } = useCadence();
    const { width, height } = useWindowDimensions();

    // Square artwork; capped by height so the controls still fit on short screens.
    const artSize = Math.round(Math.min(width - 2 * ART_INSET, height * 0.4));
    const textInset = (width - artSize) / 2 - GUTTER;

    const title = playback ? (playback.title ?? playback.trackPath.split(/[\\/]/).at(-1)) : "Nothing playing";
    const subtitle = playback
        ? (playback.artist ?? "Unknown Artist")
        : status === "connected"
          ? "Pick a track from Search to start."
          : "Connect to Cadence desktop to start.";

    const castButton = (
        <Pressable onPress={() => router.navigate("/connect")} hitSlop={10} accessibilityRole="button" accessibilityLabel="Devices">
            <Feather name="cast" size={22} color={colors.textTime} />
        </Pressable>
    );

    return (
        <View style={styles.root}>
            <Screen headerTitle="Now Playing" headerRight={castButton}>
                <Artwork size={artSize} />

                <View style={[styles.info, { paddingHorizontal: textInset }]}>
                    <Text style={type.title} numberOfLines={2}>{title}</Text>
                    <Text style={type.artist} numberOfLines={1}>{subtitle}</Text>
                </View>

                {device && status === "connected" && (
                    <Pressable
                        style={({ pressed }) => [styles.pill, pressed && styles.pressed]}
                        onPress={() => router.navigate("/connect")}
                        accessibilityRole="button"
                        accessibilityLabel={`Playing on ${device.name}. Change device`}
                    >
                        <Feather name="monitor" size={17} color={colors.accentIcon} />
                        <Text style={styles.pillText} numberOfLines={1}>Playing on {device.name}</Text>
                        <Feather name="chevron-right" size={16} color={colors.accentPillText} />
                    </Pressable>
                )}

                {playback && (
                    <>
                        <SeekBar variant="full" style={styles.seek} />

                        <View style={styles.controls}>
                            <Pressable onPress={previous} hitSlop={12} accessibilityRole="button" accessibilityLabel="Previous">
                                <Ionicons name="play-skip-back" size={30} color={colors.text} />
                            </Pressable>
                            <Pressable
                                onPress={playback.playing ? pause : resume}
                                style={({ pressed }) => [styles.play, pressed && styles.pressed]}
                                accessibilityRole="button"
                                accessibilityLabel={playback.playing ? "Pause" : "Play"}
                            >
                                <Ionicons
                                    name={playback.playing ? "pause" : "play"}
                                    size={30}
                                    color={colors.accentText}
                                    style={!playback.playing && styles.playNudge}
                                />
                            </Pressable>
                            <Pressable onPress={next} hitSlop={12} accessibilityRole="button" accessibilityLabel="Next">
                                <Ionicons name="play-skip-forward" size={30} color={colors.text} />
                            </Pressable>
                        </View>
                        <AudioMetadata
                            fileFormat={playback.fileFormat}
                            sampleRate={playback.sampleRate}
                            bitrate={playback.bitrate}
                        />
                    </>
                )}
            </Screen>
        </View>
    );
}

function AudioMetadata({ fileFormat, sampleRate, bitrate }: Pick<PlaybackState, "fileFormat" | "sampleRate" | "bitrate">) {
    const sampleRateLabel = sampleRate === null ? "—" : `${sampleRate / 1000} kHz`;
    const bitrateLabel = bitrate === null ? "—" : `${bitrate} kbps`;
    return (
        <View
            style={styles.metadata}
            accessible
            accessibilityLabel={`Format ${fileFormat ?? "unavailable"}, sample rate ${sampleRate === null ? "unavailable" : sampleRateLabel}, bitrate ${bitrate === null ? "unavailable" : bitrateLabel}`}
        >
            <View style={styles.metadataCell}>
                <Feather name="layers" size={16} color={colors.accentIcon} />
                <Text style={styles.metadataText} numberOfLines={1}>{fileFormat ?? "—"}</Text>
            </View>
            <View style={styles.metadataDivider} />
            <View style={styles.metadataCell}>
                <MaterialCommunityIcons name="waveform" size={18} color={colors.accentIcon} />
                <Text style={styles.metadataText} numberOfLines={1}>{sampleRateLabel}</Text>
            </View>
            <View style={styles.metadataDivider} />
            <View style={styles.metadataCell}>
                <Ionicons name="speedometer-outline" size={18} color={colors.textMuted} />
                <Text style={styles.metadataText} numberOfLines={1}>{bitrateLabel}</Text>
            </View>
        </View>
    );
}

/** Placeholder until the desktop serves cover art. */
function Artwork({ size }: { size: number }) {
    const disc = Math.round(size * 0.42);
    return (
        <View style={[styles.art, { width: size, height: size }]}>
            <View style={[styles.disc, { width: disc, height: disc, borderRadius: disc / 2 }]}>
                <Ionicons name="musical-notes" size={Math.round(size * 0.18)} color={colors.accent} />
            </View>
        </View>
    );
}

const styles = StyleSheet.create({
    root: { flex: 1, backgroundColor: colors.bg },
    art: {
        alignSelf: "center",
        marginTop: ART_TOP,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: "#16161C",
        alignItems: "center",
        justifyContent: "center",
    },
    disc: { backgroundColor: colors.accentSoftBg, alignItems: "center", justifyContent: "center" },
    info: { marginTop: 25 },
    pill: {
        alignSelf: "center",
        maxWidth: "100%",
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.sm,
        height: 32,
        marginTop: 20,
        paddingHorizontal: 14,
        borderRadius: radius.pill,
        backgroundColor: colors.accentSoftBg,
    },
    pillText: { flexShrink: 1, fontFamily: fonts.regular, fontSize: 12.5, color: colors.accentPillText },
    pressed: { opacity: 0.7 },
    seek: { marginTop: 9, marginHorizontal: SEEK_INSET - GUTTER },
    controls: {
        marginTop: spacing.md,
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "center",
        gap: 32,
    },
    play: {
        width: PLAY_SIZE,
        height: PLAY_SIZE,
        borderRadius: PLAY_SIZE / 2,
        backgroundColor: colors.accent,
        alignItems: "center",
        justifyContent: "center",
    },
    // Optically centre the play triangle.
    playNudge: { marginLeft: 3 },
    metadata: {
        flexDirection: "row",
        alignItems: "center",
        marginTop: spacing.xxl,
        marginHorizontal: SEEK_INSET - GUTTER,
    },
    metadataCell: {
        flex: 1,
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "center",
        gap: spacing.sm,
    },
    metadataText: { ...type.caption, flexShrink: 1 },
    metadataDivider: { width: 1, height: 18, backgroundColor: colors.track },
});
