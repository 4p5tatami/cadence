import { useRef } from "react";
import { ActivityIndicator, Animated, Easing, Pressable, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { Screen, SectionHeader } from "@/components/Screen";
import { DeviceCard } from "@/components/DeviceCard";
import { colors, radius, spacing } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";

export default function ConnectScreen() {
    const { devices, rescan, device, status, error, connect, disconnect } = useCadence();
    const others = devices.filter((d) => d.url !== device?.url);
    const connected = status === "connected";

    return (
        <Screen
            title="Connect to Desktop"
            subtitle="Play music from your PC library. Index folders on cadence desktop. Devices are discovered on your local network."
        >
            {error && (
                <View style={styles.error}>
                    <Feather name="alert-circle" size={18} color={colors.danger} />
                    <Text style={styles.errorText}>{error}</Text>
                </View>
            )}

            {device && (
                <View style={styles.section}>
                    <SectionHeader title={connected ? "Connected device" : "Connecting"} />
                    <DeviceCard device={device} state={connected ? "connected" : "connecting"} onAction={disconnect} />
                </View>
            )}

            <View style={styles.section}>
                <SectionHeader title="Discovered devices" right={<RefreshButton onPress={rescan} />} />
                {others.length === 0 ? (
                    <SearchingCard hasConnected={!!device} />
                ) : (
                    <View style={styles.list}>
                        {others.map((d) => (
                            <DeviceCard key={d.url} device={d} state="available" onAction={() => connect(d)} />
                        ))}
                    </View>
                )}
            </View>
        </Screen>
    );
}

/** Refresh icon that spins once per press. */
function RefreshButton({ onPress }: { onPress: () => void }) {
    const spin = useRef(new Animated.Value(0)).current;

    const handlePress = () => {
        spin.setValue(0);
        Animated.timing(spin, {
            toValue: 1,
            duration: 700,
            easing: Easing.out(Easing.cubic),
            useNativeDriver: true,
        }).start();
        onPress();
    };

    const rotate = spin.interpolate({ inputRange: [0, 1], outputRange: ["0deg", "360deg"] });

    return (
        <Pressable onPress={handlePress} hitSlop={12} accessibilityRole="button" accessibilityLabel="Rescan for devices">
            <Animated.View style={{ transform: [{ rotate }] }}>
                <Feather name="refresh-cw" size={22} color={colors.accent} />
            </Animated.View>
        </Pressable>
    );
}

function SearchingCard({ hasConnected }: { hasConnected: boolean }) {
    return (
        <View style={styles.searching}>
            <ActivityIndicator color={colors.accent} />
            <View style={styles.searchingText}>
                <Text style={styles.searchingTitle}>
                    {hasConnected ? "Looking for other devices…" : "Searching for devices…"}
                </Text>
                <Text style={styles.searchingHint}>
                    Make sure Cadence desktop is running on the same Wi-Fi network.
                </Text>
            </View>
        </View>
    );
}

const styles = StyleSheet.create({
    section: { marginBottom: spacing.xxl },
    list: { gap: spacing.md },
    error: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.sm,
        padding: spacing.md,
        marginBottom: spacing.xl,
        borderRadius: radius.md,
        backgroundColor: "rgba(228, 107, 107, 0.1)",
    },
    errorText: { color: colors.danger, fontSize: 14, flex: 1 },
    searching: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.lg,
        padding: spacing.lg + 2,
        borderRadius: radius.lg,
        borderWidth: 1,
        borderStyle: "dashed",
        borderColor: colors.border,
    },
    searchingText: { flex: 1, gap: 4 },
    searchingTitle: { fontSize: 16, color: colors.text },
    searchingHint: { fontSize: 13, lineHeight: 18, color: colors.textMuted },
});
