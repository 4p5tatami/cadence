import { ActivityIndicator, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { colors, radius, spacing } from "@/constants/theme";
import { PillButton } from "@/components/PillButton";
import type { DiscoveredDevice } from "@/hooks/useDiscovery";

export type DeviceCardState = "available" | "connecting" | "connected";

interface DeviceCardProps {
    device: DiscoveredDevice;
    state: DeviceCardState;
    /** Connect for "available"; cancel / disconnect otherwise. */
    onAction: () => void;
}

/** "ws://192.168.1.12:7878" -> "192.168.1.12:7878" */
function displayAddress(url: string) {
    return url.replace(/^wss?:\/\//, "");
}

const ACTION_LABEL: Record<DeviceCardState, string> = {
    available: "Connect",
    connecting: "Cancel",
    connected: "Disconnect",
};

export function DeviceCard({ device, state, onAction }: DeviceCardProps) {
    const highlighted = state !== "available";

    return (
        <View style={[styles.card, highlighted && styles.cardHighlighted]}>
            <View style={styles.iconTile}>
                {/* The desktop doesn't advertise a device type yet, so every device gets the monitor icon. */}
                <Feather name="monitor" size={26} color={colors.accent} />
            </View>

            <View style={styles.meta}>
                <Text style={styles.name} numberOfLines={1}>{device.name}</Text>
                {state === "connecting" ? (
                    <View style={styles.statusRow}>
                        <ActivityIndicator size="small" color={colors.accent} />
                        <Text style={styles.address}>Connecting…</Text>
                    </View>
                ) : (
                    <Text style={styles.address} numberOfLines={1}>{displayAddress(device.url)}</Text>
                )}
            </View>

            <PillButton
                label={ACTION_LABEL[state]}
                variant={state === "connected" ? "filled" : "outline"}
                onPress={onAction}
            />
        </View>
    );
}

const styles = StyleSheet.create({
    card: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.lg,
        paddingVertical: spacing.lg + 2,
        paddingLeft: spacing.lg,
        paddingRight: spacing.lg,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.border,
        backgroundColor: colors.surface,
    },
    cardHighlighted: {
        borderColor: colors.accentSoftBorder,
        backgroundColor: colors.accentSoftBg,
    },
    iconTile: {
        width: 56,
        height: 56,
        borderRadius: radius.md,
        backgroundColor: colors.surfaceRaised,
        alignItems: "center",
        justifyContent: "center",
    },
    meta: { flex: 1, gap: 4 },
    name: { fontSize: 17, fontWeight: "500", color: colors.text },
    address: { fontSize: 14, color: colors.textMuted },
    statusRow: { flexDirection: "row", alignItems: "center", gap: spacing.sm },
});
