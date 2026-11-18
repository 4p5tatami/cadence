import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { CadenceProvider } from "@/providers/CadenceProvider";
import { ToastProvider } from "@/providers/ToastProvider";
import { colors } from "@/constants/theme";

export default function RootLayout() {
    return (
        <SafeAreaProvider>
            <ToastProvider>
                <CadenceProvider>
                    <StatusBar style="light" />
                    <Stack screenOptions={{ headerShown: false, contentStyle: { backgroundColor: colors.bg } }} />
                </CadenceProvider>
            </ToastProvider>
        </SafeAreaProvider>
    );
}
