import { useRef } from "react";
import { ActivityIndicator, Animated, Easing, Pressable, StyleSheet, Text, View } from "react-native";
import { Feather } from "@expo/vector-icons";
import { Screen, SectionHeader } from "@/components/Screen";
import { DeviceCard } from "@/components/DeviceCard";
import { colors, fonts, radius, spacing, type } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";

export default function ConnectScreen() {
    const { devices, scanning, rescan, device, status, error, connect, disconnect } = useCadence();
    const others = devices.filter((d) => d.url !== device?.url);
    const connected = status === "connected";

    return (
        <Screen
            headerTitle="Connect"
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
                    <SearchingCard hasConnected={!!device} scanning={scanning} />
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
                <Feather name="refresh-cw" size={20} color={colors.accentIcon} />
            </Animated.View>
        </Pressable>
    );
}

/** Empty state for the discovered list: searching, or (once connected) the last manual scan found nothing. */
function SearchingCard({ hasConnected, scanning }: { hasConnected: boolean; scanning: boolean }) {
    const title = scanning
        ? hasConnected
            ? "Looking for other devices…"
            : "Searching for devices…"
        : "No other devices found.";
    const hint = scanning
        ? "Make sure Cadence desktop is running on the same Wi-Fi network."
        : "Tap refresh to search again.";

    return (
        <View style={styles.searching}>
            {scanning ? (
                <ActivityIndicator color={colors.accent} />
            ) : (
                <Feather name="wifi" size={20} color={colors.tabInactive} />
            )}
            <View style={styles.searchingText}>
                <Text style={type.body}>{title}</Text>
                <Text style={type.caption}>{hint}</Text>
            </View>
        </View>
    );
}

const styles = StyleSheet.create({
    section: { marginBottom: spacing.xxl },
    list: { gap: 10 },
    error: {
        flexDirection: "row",
        alignItems: "center",
        gap: spacing.sm,
        padding: spacing.md,
        marginBottom: spacing.xl,
        borderRadius: radius.md,
        backgroundColor: "rgba(228, 107, 107, 0.1)",
    },
    errorText: { flex: 1, fontFamily: fonts.regular, fontSize: 14, lineHeight: 20, color: colors.danger },
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
    searchingText: { flex: 1, gap: 3 },
});
