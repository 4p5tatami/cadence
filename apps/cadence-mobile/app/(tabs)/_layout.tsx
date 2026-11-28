import { Tabs } from "expo-router";
import { TabBar } from "@/components/TabBar";
import { colors } from "@/constants/theme";

export default function TabsLayout() {
    return (
        <Tabs
            tabBar={(props) => <TabBar {...props} />}
            backBehavior="history"
            screenOptions={{ headerShown: false, sceneStyle: { backgroundColor: colors.bg } }}
        >
            <Tabs.Screen name="connect" />
            <Tabs.Screen name="search" />
            <Tabs.Screen name="library" />
            <Tabs.Screen name="settings" />
            {/* Not a tab: reached from the mini player, but keeps the tab bar visible. */}
            <Tabs.Screen name="now-playing" options={{ href: null }} />
        </Tabs>
    );
}
