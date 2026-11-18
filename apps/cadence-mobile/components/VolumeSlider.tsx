import { useEffect, useRef, useState } from "react";
import { PanResponder, Pressable, StyleSheet, Text, View, type AccessibilityActionEvent } from "react-native";
import { Feather } from "@expo/vector-icons";
import { colors, spacing } from "@/constants/theme";

interface VolumeSliderProps {
    /** Level reported by the desktop, 0..1. */
    value: number;
    /** Called with the new level, throttled while dragging; the final value is always sent. */
    onChange: (level: number) => void;
    disabled?: boolean;
}

const SEND_INTERVAL_MS = 60;
/** How long to keep showing the dragged value if the desktop never echoes it back. */
const CONFIRM_TIMEOUT_MS = 1500;
const STEP = 0.05;
const DEFAULT_UNMUTE_LEVEL = 0.3;
const THUMB = 18;

const clamp = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 100) / 100;

/**
 * Pure-JS volume slider (no native module, so no dev-client rebuild).
 * Shows the dragged value optimistically until the desktop's 100 ms state
 * broadcast catches up, so the thumb doesn't jump back mid-drag.
 * Tapping the speaker icon mutes / restores the previous level.
 */
export function VolumeSlider({ value, onChange, disabled }: VolumeSliderProps) {
    const [width, setWidth] = useState(0);
    const [pending, setPending] = useState<number | null>(null);
    const shown = pending ?? value;

    // Refs so the PanResponder (created once) always sees current values.
    const widthRef = useRef(0);
    const onChangeRef = useRef(onChange);
    const disabledRef = useRef(disabled);
    const shownRef = useRef(shown);
    onChangeRef.current = onChange;
    disabledRef.current = disabled;
    shownRef.current = shown;

    const dragging = useRef(false);
    const startX = useRef(0);
    const lastSentAt = useRef(0);
    const confirmTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const lastAudible = useRef(value > 0 ? value : DEFAULT_UNMUTE_LEVEL);

    if (shown > 0) lastAudible.current = shown;

    // Drop the optimistic value once the desktop reports it back.
    useEffect(() => {
        if (pending !== null && !dragging.current && Math.abs(value - pending) < 0.005) setPending(null);
    }, [value, pending]);

    useEffect(() => () => clearTimeout(confirmTimer.current), []);

    const emit = (level: number, final: boolean) => {
        const next = clamp(level);
        setPending(next);
        const now = Date.now();
        if (final || now - lastSentAt.current >= SEND_INTERVAL_MS) {
            lastSentAt.current = now;
            onChangeRef.current(next);
        }
        if (final) {
            clearTimeout(confirmTimer.current);
            confirmTimer.current = setTimeout(() => setPending(null), CONFIRM_TIMEOUT_MS);
        }
    };
    const emitRef = useRef(emit);
    emitRef.current = emit;

    const levelAt = (x: number) => (widthRef.current > 0 ? x / widthRef.current : 0);

    const responder = useRef(
        PanResponder.create({
            onStartShouldSetPanResponder: () => !disabledRef.current,
            onMoveShouldSetPanResponder: () => !disabledRef.current,
            // Keep the gesture even if a parent ScrollView wants it.
            onPanResponderTerminationRequest: () => false,
            onPanResponderGrant: (e) => {
                dragging.current = true;
                clearTimeout(confirmTimer.current);
                startX.current = e.nativeEvent.locationX;
                emitRef.current(levelAt(startX.current), false);
            },
            onPanResponderMove: (_, g) => emitRef.current(levelAt(startX.current + g.dx), false),
            onPanResponderRelease: (_, g) => {
                dragging.current = false;
                emitRef.current(levelAt(startX.current + g.dx), true);
            },
            onPanResponderTerminate: () => {
                dragging.current = false;
                emitRef.current(shownRef.current, true);
            },
        }),
    ).current;

    const toggleMute = () => emit(shown > 0 ? 0 : lastAudible.current, true);

    const onAccessibilityAction = (e: AccessibilityActionEvent) => {
        if (e.nativeEvent.actionName === "increment") emit(shown + STEP, true);
        if (e.nativeEvent.actionName === "decrement") emit(shown - STEP, true);
    };

    const icon = shown === 0 ? "volume-x" : shown < 0.5 ? "volume-1" : "volume-2";
    const percent = Math.round(shown * 100);

    return (
        <View style={[styles.row, disabled && styles.disabled]}>
            <Pressable
                onPress={toggleMute}
                disabled={disabled}
                hitSlop={10}
                accessibilityRole="button"
                accessibilityLabel={shown > 0 ? "Mute" : "Unmute"}
            >
                <Feather name={icon} size={22} color={colors.text} />
            </Pressable>

            <View
                style={styles.hitArea}
                onLayout={(e) => {
                    widthRef.current = e.nativeEvent.layout.width;
                    setWidth(e.nativeEvent.layout.width);
                }}
                accessible
                accessibilityRole="adjustable"
                accessibilityLabel="Volume"
                accessibilityValue={{ min: 0, max: 100, now: percent }}
                accessibilityActions={[{ name: "increment" }, { name: "decrement" }]}
                onAccessibilityAction={onAccessibilityAction}
                {...responder.panHandlers}
            >
                {/* Children ignore touches so locationX is always relative to the hit area. */}
                <View style={styles.track} pointerEvents="none">
                    <View style={[styles.fill, { width: `${shown * 100}%` }]} />
                </View>
                <View
                    pointerEvents="none"
                    style={[styles.thumb, { left: Math.min(Math.max(0, shown * width - THUMB / 2), width - THUMB) }]}
                />
            </View>

            <Text style={styles.percent}>{percent}%</Text>
        </View>
    );
}

const styles = StyleSheet.create({
    row: { flexDirection: "row", alignItems: "center", gap: spacing.md },
    disabled: { opacity: 0.4 },
    hitArea: { flex: 1, height: 36, justifyContent: "center" },
    track: { height: 4, borderRadius: 2, backgroundColor: colors.divider, overflow: "hidden" },
    fill: { height: 4, backgroundColor: colors.accent },
    thumb: {
        position: "absolute",
        width: THUMB,
        height: THUMB,
        borderRadius: THUMB / 2,
        backgroundColor: colors.accent,
    },
    percent: { width: 44, textAlign: "right", fontSize: 14, color: colors.textMuted, fontVariant: ["tabular-nums"] },
});
