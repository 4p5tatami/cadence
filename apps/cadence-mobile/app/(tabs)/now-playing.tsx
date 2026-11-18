import { Pressable, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { router } from "expo-router";
import { Screen } from "@/components/Screen";
import { VolumeSlider } from "@/components/VolumeSlider";
import { colors, spacing, type } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";
import { formatMs, usePlaybackPosition } from "@/hooks/usePlaybackPosition";

export default function NowPlayingScreen() {
    const { playback, device, volume, setVolume, pause, resume, next, previous } = useCadence();
    const positionMs = usePlaybackPosition(playback);

    const headerRight = (
        <>
            <Pressable onPress={() => router.navigate("/connect")} hitSlop={8} accessibilityLabel="Devices">
                <Feather name="cast" size={22} color={colors.text} />
            </Pressable>
            {/* TODO: overflow menu */}
            <Feather name="more-horizontal" size={22} color={colors.text} />
        </>
    );

    return (
        <Screen back headerTitle="Now Playing" headerRight={headerRight}>
            {!playback ? (
                <Text style={[type.caption, styles.empty]}>Nothing is playing.</Text>
            ) : (
                <View>
                    {/* TODO: Artwork */}
                    {/* TODO: TrackInfo (title / artist / album + like) */}
                    <Text style={type.title}>{playback.title ?? playback.trackPath.split(/[\\/]/).at(-1)}</Text>
                    <Text style={type.subtitle}>{playback.artist ?? "Unknown Artist"}</Text>

                    {/* TODO: "Playing on <device>" pill */}
                    {/* TODO: Seek bar */}
                    <Text style={type.caption}>
                        {formatMs(positionMs)} / {formatMs(playback.durationMs)}
                    </Text>

                    {/* TODO: Transport controls (shuffle / prev / play-pause / next / repeat) */}
                    <View style={styles.controls}>
                        <Pressable onPress={previous}><Feather name="skip-back" size={28} color={colors.text} /></Pressable>
                        <Pressable onPress={playback.playing ? pause : resume}>
                            <Feather name={playback.playing ? "pause" : "play"} size={32} color={colors.accent} />
                        </Pressable>
                        <Pressable onPress={next}><Feather name="skip-forward" size={28} color={colors.text} /></Pressable>
                    </View>

                    <View style={styles.volume}>
                        <VolumeSlider value={volume ?? 0} disabled={volume === null} onChange={setVolume} />
                    </View>

                    {/* TODO: Footer (Queue | output device) */}
                    <Text style={type.caption}>{device?.name}</Text>
                </View>
            )}
        </Screen>
    );
}

const styles = StyleSheet.create({
    empty: { marginTop: spacing.xxl, textAlign: "center" },
    volume: { marginBottom: spacing.xl },
    controls: { flexDirection: "row", justifyContent: "center", gap: spacing.xxl, marginVertical: spacing.xl },
});
