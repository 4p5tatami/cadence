import { useCallback, useEffect, useRef, useState } from "react";
import { Animated, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Feather } from "@expo/vector-icons";
import { Slider } from "@/components/Slider";
import { colors, fonts, radius, spacing } from "@/constants/theme";
import { useScrub } from "@/hooks/useScrub";
import { useVolumeKeys } from "@/hooks/useVolumeKeys";
import { useCadence } from "@/providers/CadenceProvider";
import { useSettings } from "@/providers/SettingsProvider";

/** How long the HUD stays up after the last button press or touch. */
const HIDE_AFTER_MS = 2000;
const SEND_INTERVAL_MS = 60;
/** Presses closer together than this build on the level we last sent, not the (lagging) reported one. */
const BURST_MS = 600;

const round = (v: number) => Math.round(Math.min(1, Math.max(0, v)) * 100) / 100;

/**
 * Pill at the top of the screen showing the desktop volume. Appears when a phone
 * volume button is pressed (which steps the volume), and doubles as a slider.
 * Hides HIDE_AFTER_MS after the last press or touch; it stays up while dragging.
 */
export function VolumeHud() {
    const { status, volume, setVolume } = useCadence();
    const { settings } = useSettings();
    const insets = useSafeAreaInsets();
    const connected = status === "connected" && volume !== null;

    const { shown, start, move, end } = useScrub(volume ?? 0, { tolerance: 0.005 });
    const shownRef = useRef(shown);
    shownRef.current = shown;
    const lastSent = useRef({ level: 0, at: 0 });

    const send = useCallback(
        (level: number) => {
            lastSent.current = { level, at: Date.now() };
            setVolume(level);
        },
        [setVolume],
    );

    // --- Visibility: mount, fade in, hide timer, fade out, unmount ---
    const [mounted, setMounted] = useState(false);
    const anim = useRef(new Animated.Value(0)).current;
    const hideTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

    const hide = useCallback(() => {
        Animated.timing(anim, { toValue: 0, duration: 180, useNativeDriver: true }).start(({ finished }) => {
            if (finished) setMounted(false);
        });
    }, [anim]);

    /** Shows the HUD (if hidden) and restarts the hide countdown; `hold` pauses it, e.g. while dragging. */
    const poke = useCallback(
        (hold = false) => {
            clearTimeout(hideTimer.current);
            setMounted(true);
            anim.stopAnimation();
            Animated.timing(anim, { toValue: 1, duration: 160, useNativeDriver: true }).start();
            if (!hold) hideTimer.current = setTimeout(hide, HIDE_AFTER_MS);
        },
        [anim, hide],
    );

    useEffect(() => () => clearTimeout(hideTimer.current), []);

    // Disconnecting hides it straight away.
    useEffect(() => {
        if (!connected) {
            clearTimeout(hideTimer.current);
            anim.setValue(0);
            setMounted(false);
        }
    }, [connected, anim]);

    // --- Hardware buttons ---
    useVolumeKeys((dir) => {
        if (!connected) return;
        const recent = Date.now() - lastSent.current.at < BURST_MS;
        const base = recent ? lastSent.current.level : shownRef.current;
        const level = round(base + dir * (settings.volumeStepPercent / 100));
        start();
        end(level);
        if (level !== base) send(level);
        poke();
    });

    // --- Slider ---
    const onStart = () => {
        start();
        poke(true);
    };
    const onMove = (v: number) => {
        const level = round(v);
        move(level);
        if (Date.now() - lastSent.current.at >= SEND_INTERVAL_MS) send(level);
    };
    const onEnd = (v: number) => {
        const level = round(v);
        end(level);
        send(level);
        poke();
    };

    if (!mounted) return null;

    const percent = Math.round(shown * 100);
    const icon = shown === 0 ? "volume-x" : shown < 0.5 ? "volume-1" : "volume-2";
    const translateY = anim.interpolate({ inputRange: [0, 1], outputRange: [-12, 0] });
    const scale = anim.interpolate({ inputRange: [0, 1], outputRange: [0.94, 1] });

    return (
        <Animated.View
            pointerEvents="box-none"
            style={[styles.wrap, { top: insets.top + spacing.sm, opacity: anim, transform: [{ translateY }, { scale }] }]}
        >
            <View style={styles.pill} accessibilityLiveRegion="polite">
                <Feather name={icon} size={22} color={colors.textMuted} style={styles.icon} />
                <View style={styles.body}>
                    <Text style={styles.label}>Volume: {percent}%</Text>
                    <Slider
                        value={shown}
                        onStart={onStart}
                        onMove={onMove}
                        onEnd={onEnd}
                        thickness={5}
                        hitSize={22}
                        thumbSize={14}
                        showThumb="active"
                        trackColor="rgba(255, 255, 255, 0.16)"
                        fillColor={colors.textMuted}
                        thumbColor="#FFFFFF"
                        accessibilityLabel="Desktop volume"
                    />
                </View>
            </View>
        </Animated.View>
    );
}

const styles = StyleSheet.create({
    wrap: { position: "absolute", left: 0, right: 0, alignItems: "center" },
    pill: {
        width: 250,
        flexDirection: "row",
        alignItems: "center",
        paddingLeft: spacing.lg + 2,
        paddingRight: spacing.xl,
        paddingTop: spacing.sm + 2,
        paddingBottom: spacing.xs,
        borderRadius: radius.pill,
        backgroundColor: "rgba(28, 28, 32, 0.96)",
        borderWidth: 1,
        borderColor: colors.surfaceBorder,
        shadowColor: "#000",
        shadowOpacity: 0.35,
        shadowRadius: 12,
        shadowOffset: { width: 0, height: 4 },
        elevation: 8,
    },
    icon: { marginRight: spacing.md },
    body: { flex: 1 },
    label: { textAlign: "center", color: colors.textMuted, fontFamily: fonts.semibold, fontSize: 14, lineHeight: 18 },
});
