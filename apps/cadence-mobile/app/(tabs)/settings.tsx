import { useEffect, useState } from "react";
import { StyleSheet, Text, TextInput, View } from "react-native";
import { Screen } from "@/components/Screen";
import { colors, fonts, radius, spacing, type } from "@/constants/theme";
import { useSettings, VOLUME_STEP_MAX, VOLUME_STEP_MIN } from "@/providers/SettingsProvider";

export default function SettingsScreen() {
    const { settings, update } = useSettings();
    const saved = String(settings.volumeStepPercent);
    const [draft, setDraft] = useState(saved);

    // Follow the stored value (it loads asynchronously, and commits clamp it).
    useEffect(() => setDraft(saved), [saved]);

    const commit = () => {
        const n = parseInt(draft, 10);
        if (Number.isFinite(n)) update({ volumeStepPercent: n });
        // Invalid or clamped input snaps back to what was actually saved.
        setDraft(String(Number.isFinite(n) ? Math.min(VOLUME_STEP_MAX, Math.max(VOLUME_STEP_MIN, n)) : saved));
    };

    return (
        <Screen title="Settings">
            <View style={styles.row}>
                <View style={styles.labelBlock}>
                    <Text style={type.body}>Volume delta on button press</Text>
                    <Text style={type.caption}>
                        How much the phone's volume buttons change the desktop volume ({VOLUME_STEP_MIN}–{VOLUME_STEP_MAX}%).
                    </Text>
                </View>
                <View style={styles.field}>
                    <TextInput
                        style={styles.input}
                        value={draft}
                        onChangeText={(t) => setDraft(t.replace(/[^0-9]/g, ""))}
                        onBlur={commit}
                        onSubmitEditing={commit}
                        keyboardType="number-pad"
                        returnKeyType="done"
                        maxLength={3}
                        selectTextOnFocus
                        accessibilityLabel="Volume delta on button press, percent"
                    />
                    <Text style={styles.unit}>%</Text>
                </View>
            </View>
        </Screen>
    );
}

const styles = StyleSheet.create({
    row: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.lg,
        padding: spacing.lg,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.surfaceBorder,
        backgroundColor: colors.surface,
    },
    labelBlock: { flex: 1, gap: spacing.xs },
    field: {
        flexDirection: "row",
        alignItems: "center",
        height: 40,
        paddingHorizontal: spacing.md,
        borderRadius: radius.sm,
        backgroundColor: colors.surfaceRaised,
        borderWidth: 1,
        borderColor: colors.border,
    },
    input: {
        minWidth: 32,
        padding: 0,
        textAlign: "right",
        color: colors.text,
        fontFamily: fonts.medium,
        fontSize: 15,
    },
    unit: { marginLeft: 2, color: colors.textMuted, fontFamily: fonts.medium, fontSize: 15 },
});
