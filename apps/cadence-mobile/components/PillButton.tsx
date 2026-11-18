import { Pressable, StyleSheet, Text } from "react-native";
import { colors, radius } from "@/constants/theme";

interface PillButtonProps {
    label: string;
    onPress: () => void;
    /** "filled" = lavender fill (primary), "outline" = dark fill with lavender text. */
    variant?: "filled" | "outline";
    disabled?: boolean;
}

export function PillButton({ label, onPress, variant = "filled", disabled }: PillButtonProps) {
    const filled = variant === "filled";
    return (
        <Pressable
            onPress={onPress}
            disabled={disabled}
            accessibilityRole="button"
            accessibilityLabel={label}
            style={({ pressed }) => [
                styles.base,
                filled ? styles.filled : styles.outline,
                (pressed || disabled) && styles.dimmed,
            ]}
        >
            <Text style={[styles.label, { color: filled ? colors.accentText : colors.accent }]}>{label}</Text>
        </Pressable>
    );
}

const styles = StyleSheet.create({
    base: {
        minWidth: 116,
        height: 42,
        paddingHorizontal: 22,
        borderRadius: radius.pill,
        alignItems: "center",
        justifyContent: "center",
    },
    filled: { backgroundColor: colors.accent },
    outline: {
        backgroundColor: colors.outlineBtnBg,
        borderWidth: 1,
        borderColor: colors.outlineBtnBorder,
    },
    dimmed: { opacity: 0.6 },
    label: { fontSize: 15, fontWeight: "500" },
});
