import type { ReactNode } from "react";
import { Pressable, ScrollView, StyleSheet, Text, View } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { Feather } from "@expo/vector-icons";
import { router } from "expo-router";
import { colors, GUTTER, spacing, TEXT_INSET, type } from "@/constants/theme";

interface ScreenProps {
    /** Large page title ("Connect to Desktop"). */
    title?: string;
    /** Muted copy under the title. */
    subtitle?: string;
    /** Small centred title in the top bar ("Now Playing"). */
    headerTitle?: string;
    /** Show the back chevron in the top bar. */
    back?: boolean;
    /** Icons on the right of the top bar. */
    headerRight?: ReactNode;
    /** Wrap children in a ScrollView (default true). */
    scroll?: boolean;
    children?: ReactNode;
}

function goBack() {
    if (router.canGoBack()) router.back();
    else router.replace("/connect");
}

/**
 * Page scaffold matching the mockups: a fixed-height top bar (so titles line up
 * across screens), then title + subtitle, then content. Cards span the GUTTER;
 * text blocks sit a further TEXT_INSET in. The bottom edge is left to the tab bar.
 */
export function Screen({ title, subtitle, headerTitle, back, headerRight, scroll = true, children }: ScreenProps) {
    const body = (
        <>
            {(title || subtitle) && (
                <View style={styles.titleBlock}>
                    {title && <Text style={type.title}>{title}</Text>}
                    {subtitle && <Text style={[type.subtitle, styles.subtitle]}>{subtitle}</Text>}
                </View>
            )}
            {children}
        </>
    );

    return (
        <SafeAreaView edges={["top"]} style={styles.root}>
            <View style={styles.topBar}>
                {/* Absolutely centred so it stays centred whatever sits on either side. */}
                {headerTitle && (
                    <Text style={[type.headerTitle, styles.headerTitle]} numberOfLines={1} pointerEvents="none">
                        {headerTitle}
                    </Text>
                )}
                <View style={styles.topBarSide}>
                    {back && (
                        <Pressable onPress={goBack} hitSlop={12} accessibilityRole="button" accessibilityLabel="Back">
                            <Feather name="chevron-left" size={28} color={colors.text} />
                        </Pressable>
                    )}
                </View>
                <View style={[styles.topBarSide, styles.topBarRight]}>{headerRight}</View>
            </View>

            {scroll ? (
                <ScrollView
                    contentContainerStyle={styles.content}
                    showsVerticalScrollIndicator={false}
                    // Let taps reach buttons/rows while the keyboard is open instead of only dismissing it.
                    keyboardShouldPersistTaps="handled"
                >
                    {body}
                </ScrollView>
            ) : (
                <View style={[styles.content, styles.fill]}>{body}</View>
            )}
        </SafeAreaView>
    );
}

/** Section heading row ("Connected device", "Discovered devices" + action). */
export function SectionHeader({ title, right }: { title: string; right?: ReactNode }) {
    return (
        <View style={styles.sectionHeader}>
            <Text style={type.section}>{title}</Text>
            {right}
        </View>
    );
}

const styles = StyleSheet.create({
    root: { flex: 1, backgroundColor: colors.bg },
    fill: { flex: 1 },
    topBar: {
        height: 52,
        paddingHorizontal: GUTTER + TEXT_INSET - 4,
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
    },
    headerTitle: { position: "absolute", left: 72, right: 72, textAlign: "center" },
    topBarSide: { minWidth: 44, flexDirection: "row", alignItems: "center", gap: spacing.lg },
    topBarRight: { justifyContent: "flex-end" },
    content: { paddingHorizontal: GUTTER, paddingBottom: spacing.xl },
    titleBlock: { marginTop: spacing.lg, marginBottom: 28, paddingHorizontal: TEXT_INSET },
    subtitle: { marginTop: spacing.xs },
    sectionHeader: {
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        marginBottom: spacing.md,
        paddingHorizontal: TEXT_INSET,
    },
});
