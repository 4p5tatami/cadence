import { Pressable, StyleSheet, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Feather, Ionicons } from "@expo/vector-icons";
import { router } from "expo-router";
import { Screen } from "@/components/Screen";
import { SeekBar } from "@/components/SeekBar";
import { VolumeControl } from "@/components/VolumeControl";
import { colors, fonts, GUTTER, radius, spacing, type } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";

// Layout measured from the Now Playing mockup (390 pt wide screen).
const TOP_BAR = 52;
const ART_INSET = 34; // artwork edge to screen edge
const ART_TOP = 12;
const SEEK_INSET = 29; // seek bar edge to screen edge
const PLAY_SIZE = 66;
const VOLUME_HEIGHT = 196;

/** Now Playing. Shuffle/repeat, queue, like and the overflow menu are left out until the desktop supports them. */
export default function NowPlayingScreen() {
    const { playback, device, status, volume, setVolume, pause, resume, next, previous } = useCadence();
    const { width, height } = useWindowDimensions();
    const insets = useSafeAreaInsets();

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
                    </>
                )}
            </Screen>

            {/* Floats over the right edge, centred on the artwork, so it doesn't push the layout around. */}
            {status === "connected" && (
                <VolumeControl
                    value={volume ?? 0}
                    disabled={volume === null}
                    onChange={setVolume}
                    style={[styles.volume, { top: insets.top + TOP_BAR + ART_TOP + (artSize - VOLUME_HEIGHT) / 2 }]}
                />
            )}
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
    volume: { position: "absolute", right: 6, height: VOLUME_HEIGHT },
});
