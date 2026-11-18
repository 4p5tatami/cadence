import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { View } from "react-native";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { useFonts } from "expo-font";
// Per-weight imports so only these three files are bundled, not all 18 Inter styles.
import { Inter_400Regular } from "@expo-google-fonts/inter/400Regular";
import { Inter_500Medium } from "@expo-google-fonts/inter/500Medium";
import { Inter_600SemiBold } from "@expo-google-fonts/inter/600SemiBold";
import { CadenceProvider } from "@/providers/CadenceProvider";
import { ToastProvider } from "@/providers/ToastProvider";
import { colors } from "@/constants/theme";

export default function RootLayout() {
    const [fontsLoaded, fontError] = useFonts({ Inter_400Regular, Inter_500Medium, Inter_600SemiBold });

    // Fonts load from the bundle in a few ms; hold a blank screen rather than flash system text.
    // On a load error, carry on with the system font.
    if (!fontsLoaded && !fontError) return <View style={{ flex: 1, backgroundColor: colors.bg }} />;

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
