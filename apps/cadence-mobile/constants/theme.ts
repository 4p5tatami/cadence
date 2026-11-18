/**
 * Design tokens. Values are sampled from the "Connect to Desktop" mockup,
 * which is the source of truth for screen layout and styling.
 */
export const colors = {
    bg: "#0B0B0F",
    surface: "#15151B", // cards, mini player
    surfaceRaised: "#1D1D25", // icon tiles inside cards
    border: "#26262F",
    divider: "#2E2E38",

    text: "#F4F4F6",
    textMuted: "#9C9CA6",
    textDim: "#6C6C76",

    accent: "#A898FF", // lavender: primary buttons, active tab, icons
    accentText: "#15121F", // text on filled accent buttons
    accentSoftBg: "rgba(168, 152, 255, 0.07)", // "connected" card fill
    accentSoftBorder: "rgba(168, 152, 255, 0.55)", // "connected" card border
    outlineBtnBg: "#191826",
    outlineBtnBorder: "#3A3858",

    danger: "#E46B6B",
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
    md: 12, // icon tiles, mini player artwork
    lg: 16, // cards
    pill: 999,
} as const;

export const type = {
    title: { fontSize: 30, fontWeight: "600" as const, color: colors.text, letterSpacing: -0.3 },
    subtitle: { fontSize: 16, lineHeight: 23, color: colors.textMuted },
    section: { fontSize: 19, fontWeight: "500" as const, color: colors.text },
    body: { fontSize: 17, fontWeight: "500" as const, color: colors.text },
    caption: { fontSize: 14, color: colors.textMuted },
    tabLabel: { fontSize: 12 },
} as const;

/** Horizontal page gutter used by every screen. */
export const GUTTER = 20;
