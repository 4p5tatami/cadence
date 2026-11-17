const { AndroidConfig, withAndroidManifest, withAppBuildGradle } = require("expo/config-plugins");

function androidVersionCode(version) {
    const match = /^(\d+)\.(\d{1,2})\.(\d{1,2})$/.exec(version ?? "");
    if (!match) {
        throw new Error(`Expected an Android release version like 1.2.3, got ${version}`);
    }
    const [major, minor, patch] = match.slice(1).map(Number);
    return major * 10000 + minor * 100 + patch;
}

function addReleaseSigning(gradle) {
    if (gradle.includes("CADENCE_UPLOAD_STORE_FILE")) return gradle;

    const withSigningConfig = gradle.replace(/signingConfigs\s*\{/, (match) => `${match}
        release {
            if (findProperty('CADENCE_UPLOAD_STORE_FILE')) {
                storeFile file(findProperty('CADENCE_UPLOAD_STORE_FILE'))
                storePassword findProperty('CADENCE_UPLOAD_STORE_PASSWORD')
                keyAlias findProperty('CADENCE_UPLOAD_KEY_ALIAS')
                keyPassword findProperty('CADENCE_UPLOAD_KEY_PASSWORD')
            }
        }`);
    const withReleaseSigning = withSigningConfig.replace(
        /(buildTypes\s*\{[\s\S]*?\brelease\s*\{[\s\S]*?)signingConfig signingConfigs\.debug/,
        "$1signingConfig findProperty('CADENCE_UPLOAD_STORE_FILE') ? signingConfigs.release : signingConfigs.debug",
    );

    if (withSigningConfig === gradle || withReleaseSigning === withSigningConfig) {
        throw new Error("Could not configure release signing in Expo's generated app/build.gradle");
    }
    return withReleaseSigning;
}

module.exports = function withAndroidRelease(config) {
    config.android ??= {};
    config.android.versionCode ??= androidVersionCode(config.version);

    withAndroidManifest(config, (config) => {
        // Desktop discovery connects over ws:// at the local address it finds.
        const application = AndroidConfig.Manifest.getMainApplicationOrThrow(config.modResults);
        application.$["android:usesCleartextTraffic"] = "true";
        return config;
    });

    return withAppBuildGradle(config, (config) => {
        if (config.modResults.language !== "groovy") {
            throw new Error("Expected a Groovy app/build.gradle for Android release signing");
        }
        config.modResults.contents = addReleaseSigning(config.modResults.contents);
        return config;
    });
};
