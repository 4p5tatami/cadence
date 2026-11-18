import { useRef, useState } from "react";
import {
    PanResponder,
    StyleSheet,
    View,
    type AccessibilityActionEvent,
    type LayoutChangeEvent,
    type StyleProp,
    type ViewStyle,
} from "react-native";

interface SliderProps {
    /** Displayed position, 0..1. */
    value: number;
    onStart?: () => void;
    /** Every move while dragging. */
    onMove?: (v: number) => void;
    /** Release, tap, or accessibility increment/decrement. */
    onEnd: (v: number) => void;
    vertical?: boolean;
    disabled?: boolean;
    /** Track thickness. */
    thickness?: number;
    /** Touch area across the track; the visible track is centred in it. */
    hitSize?: number;
    thumbSize?: number;
    /** "active" shows the thumb only while dragging. */
    showThumb?: "always" | "active" | "never";
    trackColor: string;
    fillColor: string;
    thumbColor?: string;
    /** Sizes the slider along its axis (e.g. `{ flex: 1 }` or a fixed height). */
    style?: StyleProp<ViewStyle>;
    accessibilityLabel: string;
    /** Step for screen-reader increment/decrement. */
    step?: number;
    /** Spoken value, e.g. "1:24 of 3:53". Defaults to a percentage. */
    accessibilityText?: string;
}

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

/**
 * Pure-JS slider (no native module): tap to jump, drag to scrub, horizontal or
 * vertical. Stateless about the value itself; pair it with `useScrub` so the
 * thumb holds its position until the desktop confirms the change.
 */
export function Slider({
    value,
    onStart,
    onMove,
    onEnd,
    vertical = false,
    disabled = false,
    thickness = 4,
    hitSize = 32,
    thumbSize = 14,
    showThumb = "always",
    trackColor,
    fillColor,
    thumbColor = fillColor,
    style,
    accessibilityLabel,
    step = 0.05,
    accessibilityText,
}: SliderProps) {
    const [length, setLength] = useState(0);
    const [active, setActive] = useState(false);

    // The PanResponder is created once; refs give it current props.
    const props = useRef({ onStart, onMove, onEnd, disabled, vertical, length });
    props.current = { onStart, onMove, onEnd, disabled, vertical, length };
    const startPos = useRef(0);
    const lastLevel = useRef(0);

    const levelAt = (pos: number) => {
        const { length, vertical } = props.current;
        if (length <= 0) return 0;
        const v = clamp01(pos / length);
        return vertical ? 1 - v : v;
    };

    const responder = useRef(
        PanResponder.create({
            onStartShouldSetPanResponder: () => !props.current.disabled,
            onMoveShouldSetPanResponder: () => !props.current.disabled,
            // Keep the gesture even if a parent ScrollView wants it.
            onPanResponderTerminationRequest: () => false,
            onPanResponderGrant: (e) => {
                startPos.current = props.current.vertical ? e.nativeEvent.locationY : e.nativeEvent.locationX;
                lastLevel.current = levelAt(startPos.current);
                setActive(true);
                props.current.onStart?.();
                props.current.onMove?.(lastLevel.current);
            },
            onPanResponderMove: (_, g) => {
                lastLevel.current = levelAt(startPos.current + (props.current.vertical ? g.dy : g.dx));
                props.current.onMove?.(lastLevel.current);
            },
            onPanResponderRelease: (_, g) => {
                setActive(false);
                props.current.onEnd(levelAt(startPos.current + (props.current.vertical ? g.dy : g.dx)));
            },
            onPanResponderTerminate: () => {
                setActive(false);
                props.current.onEnd(lastLevel.current);
            },
        }),
    ).current;

    const onLayout = (e: LayoutChangeEvent) => {
        const { width, height } = e.nativeEvent.layout;
        setLength(vertical ? height : width);
    };

    const onAccessibilityAction = (e: AccessibilityActionEvent) => {
        if (e.nativeEvent.actionName === "increment") onEnd(clamp01(value + step));
        if (e.nativeEvent.actionName === "decrement") onEnd(clamp01(value - step));
    };

    const v = clamp01(value);
    const thumbVisible = showThumb === "always" || (showThumb === "active" && active);
    const thumbOffset = Math.min(Math.max(0, v * length - thumbSize / 2), Math.max(0, length - thumbSize));
    const crossOffset = (hitSize - thumbSize) / 2;

    return (
        <View
            style={[vertical ? { width: hitSize } : { height: hitSize }, styles.hit, disabled && styles.disabled, style]}
            onLayout={onLayout}
            accessible
            accessibilityRole="adjustable"
            accessibilityLabel={accessibilityLabel}
            accessibilityValue={accessibilityText ? { text: accessibilityText } : { min: 0, max: 100, now: Math.round(v * 100) }}
            accessibilityActions={[{ name: "increment" }, { name: "decrement" }]}
            onAccessibilityAction={onAccessibilityAction}
            {...responder.panHandlers}
        >
            {/* Children ignore touches so locationX/Y are always relative to the hit area. */}
            <View
                pointerEvents="none"
                style={[
                    styles.track,
                    { backgroundColor: trackColor, borderRadius: thickness / 2 },
                    vertical
                        ? { width: thickness, top: 0, bottom: 0, left: (hitSize - thickness) / 2 }
                        : { height: thickness, left: 0, right: 0, top: (hitSize - thickness) / 2 },
                ]}
            >
                <View
                    style={[
                        styles.fill,
                        { backgroundColor: fillColor },
                        vertical ? { left: 0, right: 0, bottom: 0, height: `${v * 100}%` } : { top: 0, bottom: 0, left: 0, width: `${v * 100}%` },
                    ]}
                />
            </View>
            {thumbVisible && (
                <View
                    pointerEvents="none"
                    style={[
                        styles.thumb,
                        { width: thumbSize, height: thumbSize, borderRadius: thumbSize / 2, backgroundColor: thumbColor },
                        vertical ? { bottom: thumbOffset, left: crossOffset } : { left: thumbOffset, top: crossOffset },
                    ]}
                />
            )}
        </View>
    );
}

const styles = StyleSheet.create({
    hit: { position: "relative" },
    disabled: { opacity: 0.4 },
    track: { position: "absolute", overflow: "hidden" },
    fill: { position: "absolute" },
    thumb: { position: "absolute" },
});
