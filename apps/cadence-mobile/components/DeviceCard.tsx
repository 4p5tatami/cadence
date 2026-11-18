import { ActivityIndicator, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { colors, radius, spacing, type } from "@/constants/theme";
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
            <View style={[styles.iconTile, highlighted && styles.iconTileHighlighted]}>
                {/* The desktop doesn't advertise a device type yet, so every device gets the monitor icon. */}
                <Feather name="monitor" size={26} color={colors.accentIcon} />
            </View>

            <View style={styles.meta}>
                <Text style={type.body} numberOfLines={1}>{device.name}</Text>
                {state === "connecting" ? (
                    <View style={styles.statusRow}>
                        <ActivityIndicator size="small" color={colors.accent} />
                        <Text style={type.caption}>Connecting…</Text>
                    </View>
                ) : (
                    <Text style={type.caption} numberOfLines={1}>{displayAddress(device.url)}</Text>
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

// Measured from the mockup: 81 pt tall card, 50 pt icon tile, 15 pt between tile and text.
const styles = StyleSheet.create({
    card: {
        flexDirection: "row",
        alignItems: "center",
        gap: 15,
        paddingVertical: 14,
        paddingLeft: 13,
        paddingRight: 12,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderColor: colors.surfaceBorder,
        backgroundColor: colors.surface,
    },
    cardHighlighted: {
        borderColor: colors.accentSoftBorder,
        backgroundColor: colors.accentSoftBg,
    },
    iconTile: {
        width: 50,
        height: 50,
        borderRadius: radius.md,
        backgroundColor: colors.surfaceRaised,
        alignItems: "center",
        justifyContent: "center",
    },
    iconTileHighlighted: { backgroundColor: colors.accentTile },
    meta: { flex: 1, gap: 3 },
    statusRow: { flexDirection: "row", alignItems: "center", gap: spacing.sm },
});
