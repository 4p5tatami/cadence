import { useRef } from "react";
import { Pressable, StyleSheet, View, type StyleProp, type ViewStyle } from "react-native";
import { Feather } from "@expo/vector-icons";
import { Slider } from "@/components/Slider";
import { colors, radius } from "@/constants/theme";
import { useScrub } from "@/hooks/useScrub";

interface VolumeControlProps {
    /** Level reported by the desktop, 0..1. */
    value: number;
    /** Sends a new level; called at most every SEND_INTERVAL_MS while dragging, and always on release. */
    onChange: (level: number) => void;
    disabled?: boolean;
    /** Positioning, e.g. absolute on the right edge. */
    style?: StyleProp<ViewStyle>;
}

const SEND_INTERVAL_MS = 60;
const DEFAULT_UNMUTE_LEVEL = 0.3;
const round = (v: number) => Math.round(v * 100) / 100;

/**
 * Slim vertical volume bar meant to float over a screen's edge. Semi-transparent
 * so it doesn't compete with the content underneath. Drag or tap the track to
 * set the level; tap the speaker to mute / restore the previous level.
 */
export function VolumeControl({ value, onChange, disabled, style }: VolumeControlProps) {
    const { shown, start, move, end } = useScrub(value, { tolerance: 0.005 });
    const lastSentAt = useRef(0);
    const lastAudible = useRef(value > 0 ? value : DEFAULT_UNMUTE_LEVEL);
    if (shown > 0) lastAudible.current = shown;

    const onMove = (v: number) => {
        const level = round(v);
        move(level);
        const now = Date.now();
        if (now - lastSentAt.current >= SEND_INTERVAL_MS) {
            lastSentAt.current = now;
            onChange(level);
        }
    };

    const onEnd = (v: number) => {
        const level = round(v);
        end(level);
        lastSentAt.current = Date.now();
        onChange(level);
    };

    const toggleMute = () => {
        start();
        onEnd(shown > 0 ? 0 : lastAudible.current);
    };

    const icon = shown === 0 ? "volume-x" : shown < 0.5 ? "volume-1" : "volume-2";

    return (
        <View style={[styles.wrap, disabled && styles.disabled, style]}>
            <Pressable
                onPress={toggleMute}
                disabled={disabled}
                hitSlop={8}
                accessibilityRole="button"
                accessibilityLabel={shown > 0 ? "Mute" : "Unmute"}
            >
                <Feather name={icon} size={18} color="rgba(255, 255, 255, 0.85)" />
            </Pressable>
            <Slider
                vertical
                value={shown}
                onStart={start}
                onMove={onMove}
                onEnd={onEnd}
                disabled={disabled}
                thickness={4}
                hitSize={40}
                thumbSize={14}
                trackColor="rgba(255, 255, 255, 0.14)"
                fillColor={colors.accent}
                thumbColor="#FFFFFF"
                style={styles.slider}
                accessibilityLabel="Volume"
            />
        </View>
    );
}

const styles = StyleSheet.create({
    wrap: {
        width: 40,
        height: 196,
        paddingTop: 12,
        paddingBottom: 10,
        gap: 8,
        alignItems: "center",
        borderRadius: radius.pill,
        borderWidth: 1,
        borderColor: "rgba(255, 255, 255, 0.08)",
        backgroundColor: "rgba(24, 24, 30, 0.55)",
    },
    disabled: { opacity: 0.4 },
    slider: { flex: 1 },
});
