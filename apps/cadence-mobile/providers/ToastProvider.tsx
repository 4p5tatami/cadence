import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Animated, Pressable, StyleSheet, Text } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Feather } from "@expo/vector-icons";
import { colors, radius, spacing } from "@/constants/theme";

const VISIBLE_MS = 4500;

interface Toast {
    id: number;
    message: string;
}

const ToastContext = createContext<((message: string) => void) | null>(null);

/** App-wide toast host. Shows one error toast at a time at the top of the screen. */
export function ToastProvider({ children }: { children: ReactNode }) {
    const [toast, setToast] = useState<Toast | null>(null);
    const nextId = useRef(1);

    const show = useCallback((message: string) => {
        setToast({ id: nextId.current++, message });
    }, []);

    // Stable so the toast's auto-hide timer isn't restarted by re-renders.
    const clear = useCallback(() => setToast(null), []);

    return (
        <ToastContext.Provider value={show}>
            {children}
            {toast && <ToastView key={toast.id} message={toast.message} onDone={clear} />}
        </ToastContext.Provider>
    );
}

export function useToast() {
    const show = useContext(ToastContext);
    if (!show) throw new Error("useToast must be used inside <ToastProvider>");
    return useMemo(() => ({ show }), [show]);
}

function ToastView({ message, onDone }: { message: string; onDone: () => void }) {
    const insets = useSafeAreaInsets();
    const anim = useRef(new Animated.Value(0)).current;

    const hide = useCallback(() => {
        Animated.timing(anim, { toValue: 0, duration: 180, useNativeDriver: true }).start(({ finished }) => {
            if (finished) onDone();
        });
    }, [anim, onDone]);

    useEffect(() => {
        Animated.timing(anim, { toValue: 1, duration: 220, useNativeDriver: true }).start();
        const t = setTimeout(hide, VISIBLE_MS);
        return () => clearTimeout(t);
    }, [anim, hide]);

    const translateY = anim.interpolate({ inputRange: [0, 1], outputRange: [-16, 0] });

    return (
        <Animated.View
            pointerEvents="box-none"
            style={[styles.wrap, { top: insets.top + spacing.sm, opacity: anim, transform: [{ translateY }] }]}
        >
            <Pressable style={styles.toast} onPress={hide} accessibilityRole="alert" accessibilityLiveRegion="polite">
                <Feather name="alert-circle" size={20} color={colors.danger} />
                <Text style={styles.text}>{message}</Text>
            </Pressable>
        </Animated.View>
    );
}

const styles = StyleSheet.create({
    wrap: { position: "absolute", left: spacing.md, right: spacing.md },
    toast: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.md,
        paddingVertical: spacing.md + 2,
        paddingHorizontal: spacing.lg,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: "rgba(228, 107, 107, 0.35)",
        backgroundColor: "#1F1518",
        elevation: 8,
        shadowColor: "#000",
        shadowOpacity: 0.4,
        shadowRadius: 12,
        shadowOffset: { width: 0, height: 4 },
    },
    text: { flex: 1, fontSize: 14, lineHeight: 20, color: colors.text },
});
