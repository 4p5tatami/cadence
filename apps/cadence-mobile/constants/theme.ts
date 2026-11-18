import type { TextStyle } from "react-native";

/**
 * Design tokens, measured from the "Connect to Desktop" and "Now Playing" mockups
 * (390 pt wide; 1.64 px per pt in the exported PNGs).
 */
export const colors = {
    bg: "#0F0F11",

    // Cards
    surface: "#181819", // discovered-device cards, mini player
    surfaceBorder: "#202123",
    surfaceRaised: "#1F1F23", // icon tile on a plain card
    border: "#26262F",
    divider: "#1F2022", // tab bar top edge

    // Text
    text: "#FFFFFF",
    textMuted: "#A9B2C3", // subtitles, addresses
    textHeader: "#B6BDCC", // small centred top-bar title
    textArtist: "#C2C8D4",
    textTime: "#E4E6EE",
    tabInactive: "#9AA3B3",

    // Accent (lavender)
    accent: "#A99BFD",
    accentIcon: "#A89DF7",
    accentText: "#141220", // text/icons on a filled accent background
    accentLabel: "#B5B2FD", // text on outline buttons
    accentSoftBg: "#1C1D27", // connected card, "Playing on" pill
    accentSoftBorder: "#7C7FBD", // connected card border
    accentTile: "#25243A", // icon tile on the connected card
    accentPillText: "#C1C3F5",
    outlineBtnBg: "#1F202C",
    outlineBtnBorder: "#3D4062",

    // Sliders
    track: "#2F323A",

    danger: "#E46B6B",
} as const;

/** Inter, loaded in app/_layout.tsx. Use the family per weight: Android ignores fontWeight for custom fonts. */
export const fonts = {
    regular: "Inter_400Regular",
    medium: "Inter_500Medium",
    semibold: "Inter_600SemiBold",
} as const;

export const spacing = {
    xs: 4,
    sm: 8,
    md: 12,
    lg: 16,
    xl: 24,
    xxl: 32,
} as const;

export const radius = {
    sm: 8,
    md: 10, // icon tiles
    lg: 12, // cards, artwork
    pill: 999,
} as const;

const t = (style: TextStyle) => style;

export const type = {
    /** Small centred title in the top bar ("Connect", "Now Playing"). */
    headerTitle: t({ fontFamily: fonts.regular, fontSize: 15, lineHeight: 20, color: colors.textHeader }),
    /** Large page title ("Connect to Desktop"); also the track title on Now Playing. */
    title: t({ fontFamily: fonts.semibold, fontSize: 24, lineHeight: 30, letterSpacing: -0.4, color: colors.text }),
    subtitle: t({ fontFamily: fonts.regular, fontSize: 15, lineHeight: 21, color: colors.textMuted }),
    /** Section heading ("Connected device"). */
    section: t({ fontFamily: fonts.medium, fontSize: 17, lineHeight: 22, color: colors.text }),
    /** Primary line in a list row or card ("Studio PC"). */
    body: t({ fontFamily: fonts.medium, fontSize: 15.5, lineHeight: 20, color: colors.text }),
    /** Secondary line in a list row or card ("192.168.1.12:7878"). */
    caption: t({ fontFamily: fonts.regular, fontSize: 13, lineHeight: 18, color: colors.textMuted }),
    button: t({ fontFamily: fonts.medium, fontSize: 12.5, lineHeight: 16 }),
    tabLabel: t({ fontFamily: fonts.regular, fontSize: 12, lineHeight: 16 }),
    artist: t({ fontFamily: fonts.regular, fontSize: 17, lineHeight: 22, color: colors.textArtist }),
    time: t({ fontFamily: fonts.regular, fontSize: 12, lineHeight: 16, color: colors.textTime, fontVariant: ["tabular-nums"] }),
} as const;

/** Horizontal inset for cards. Text blocks (titles, section headings) sit a further TEXT_INSET in. */
export const GUTTER = 16;
export const TEXT_INSET = 8;
