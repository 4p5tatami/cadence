import { StyleSheet, Text, View, type StyleProp, type ViewStyle } from "react-native";
import { Slider } from "@/components/Slider";
import { colors, type } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";
import { formatMs, usePlaybackPosition } from "@/hooks/usePlaybackPosition";
import { useScrub } from "@/hooks/useScrub";

interface SeekBarProps {
    /** "full": Now Playing bar with thumb and times. "mini": thin mini-player line, thumb only while dragging. */
    variant: "full" | "mini";
    style?: StyleProp<ViewStyle>;
}

/**
 * Seekable progress for the current track. Dragging previews the position
 * locally and seeks once on release (each seek reopens the decoder on the desktop).
 */
export function SeekBar({ variant, style }: SeekBarProps) {
    const { playback, seek } = useCadence();
    const positionMs = usePlaybackPosition(playback, variant === "mini" ? 500 : 250);
    const durationMs = playback?.durationMs ?? 0;
    const fraction = durationMs ? positionMs / durationMs : 0;
    // The desktop reports the new position within ~100 ms; 1.5 s of slack covers playback moving on.
    const { shown, start, move, end } = useScrub(fraction, {
        tolerance: durationMs ? 1500 / durationMs : 0.01,
        timeoutMs: 2500,
    });

    if (!playback) return null;

    const onEnd = (v: number) => {
        end(v);
        if (durationMs) seek(Math.round(v * durationMs));
    };

    const full = variant === "full";
    const shownMs = shown * durationMs;

    return (
        <View style={style}>
            <Slider
                value={shown}
                onStart={start}
                onMove={move}
                onEnd={onEnd}
                disabled={!durationMs}
                thickness={full ? 4 : 3}
                hitSize={full ? 28 : 22}
                thumbSize={full ? 12 : 11}
                showThumb={full ? "always" : "active"}
                trackColor={full ? colors.track : "#2C2D33"}
                fillColor={full ? colors.accent : colors.text}
                accessibilityLabel="Seek"
                step={durationMs ? 10_000 / durationMs : 0.05}
                accessibilityText={`${formatMs(shownMs)} of ${formatMs(durationMs)}`}
            />
            {full && (
                <View style={styles.times}>
                    <Text style={type.time}>{formatMs(shownMs)}</Text>
                    <Text style={type.time}>{formatMs(durationMs)}</Text>
                </View>
            )}
        </View>
    );
}

const styles = StyleSheet.create({
    times: { flexDirection: "row", justifyContent: "space-between", marginTop: -4 },
});
