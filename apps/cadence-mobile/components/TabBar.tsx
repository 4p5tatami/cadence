import type { ComponentProps } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Feather } from "@expo/vector-icons";
import type { BottomTabBarProps } from "@react-navigation/bottom-tabs";
import { colors, spacing, type } from "@/constants/theme";
import { MiniPlayer } from "@/components/MiniPlayer";

type IconName = ComponentProps<typeof Feather>["name"];

/** Visible tabs, in order. Route names match files in app/(tabs)/. */
const TABS: { name: string; label: string; icon: IconName }[] = [
    { name: "connect", label: "Connect", icon: "cast" },
    { name: "search", label: "Search", icon: "search" },
    { name: "library", label: "Library", icon: "music" },
];

/** Routes that hide the mini-player overlay (they show full controls themselves). */
const HIDE_MINI_PLAYER = new Set(["now-playing"]);

/**
 * Bottom panel: optional mini player overlay stacked on top of the tab bar.
 * Rendered as the Tabs `tabBar`, so screen content is laid out above it.
 */
export function TabBar({ state, navigation }: BottomTabBarProps) {
    const insets = useSafeAreaInsets();
    const current = state.routes[state.index]?.name;

    return (
        <View style={styles.root}>
            {!HIDE_MINI_PLAYER.has(current) && <MiniPlayer />}

            <View style={[styles.bar, { paddingBottom: Math.max(insets.bottom, spacing.md) }]}>
                {TABS.map((tab) => {
                    const focused = current === tab.name;
                    const color = focused ? colors.accent : colors.tabInactive;
                    return (
                        <Pressable
                            key={tab.name}
                            style={styles.item}
                            onPress={() => {
                                if (!focused) navigation.navigate(tab.name);
                            }}
                            accessibilityRole="tab"
                            accessibilityState={{ selected: focused }}
                            accessibilityLabel={tab.label}
                        >
                            <Feather name={tab.icon} size={24} color={color} />
                            <Text style={[styles.label, { color }]}>{tab.label}</Text>
                        </Pressable>
                    );
                })}
            </View>
        </View>
    );
}

const styles = StyleSheet.create({
    root: { backgroundColor: colors.bg },
    bar: {
        flexDirection: "row",
        paddingTop: spacing.md,
        borderTopWidth: StyleSheet.hairlineWidth,
        borderTopColor: colors.divider,
        backgroundColor: colors.bg,
    },
    item: { flex: 1, alignItems: "center", gap: spacing.xs },
    label: type.tabLabel,
});
