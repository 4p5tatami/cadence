import { useEffect, useState } from "react";
import { Pressable, StyleSheet, Text, TextInput, View } from "react-native";
import { Screen } from "@/components/Screen";
import { colors, radius, spacing, type } from "@/constants/theme";
import { useCadence } from "@/providers/CadenceProvider";

export default function SearchScreen() {
    const { status, searchResults, search, play } = useCadence();
    const [query, setQuery] = useState("");

    // Debounced search
    useEffect(() => {
        const t = setTimeout(() => search(query), 300);
        return () => clearTimeout(t);
    }, [query, search]);

    return (
        <Screen title="Search">
            {/* TODO: SearchField with leading icon */}
            <TextInput
                style={styles.input}
                value={query}
                onChangeText={setQuery}
                placeholder="Search for songs, albums, artists…"
                placeholderTextColor={colors.textDim}
                autoCapitalize="none"
                autoCorrect={false}
                editable={status === "connected"}
            />

            {status !== "connected" && <Text style={type.caption}>Connect to a desktop to search its library.</Text>}

            {/* TODO: TrackRow list; recent searches when query is empty */}
            <View>
                {searchResults.map((t) => (
                    <Pressable key={t.id} onPress={() => play(t.path)} style={styles.row}>
                        <Text style={type.body} numberOfLines={1}>{t.title}</Text>
                        <Text style={type.caption} numberOfLines={1}>{t.artist}</Text>
                    </Pressable>
                ))}
            </View>
        </Screen>
    );
}

const styles = StyleSheet.create({
    input: {
        height: 52,
        paddingHorizontal: spacing.lg,
        marginBottom: spacing.xl,
        borderRadius: radius.lg,
        backgroundColor: colors.surface,
        color: colors.text,
        fontSize: 16,
    },
    row: { paddingVertical: spacing.md },
});
