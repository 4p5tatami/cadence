const { withMainActivity } = require("expo/config-plugins");

const MARKER = "// cadence: volume keys";

const IMPORTS = ["android.view.KeyEvent", "com.github.kevinejohn.keyevent.KeyEventModule"];

// Volume keys go to JS (hooks/useVolumeKeys.ts) to drive the desktop volume, and are
// consumed so the phone's own volume doesn't change too. Other keys keep their default handling.
const OVERRIDES = `
  ${MARKER}
  private fun isVolumeKey(keyCode: Int) =
      keyCode == KeyEvent.KEYCODE_VOLUME_UP || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN

  override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
    val module = KeyEventModule.getInstance()
    if (module == null || !isVolumeKey(keyCode)) return super.onKeyDown(keyCode, event)
    module.onKeyDownEvent(keyCode, event)
    return true
  }

  override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
    val module = KeyEventModule.getInstance()
    if (module == null || !isVolumeKey(keyCode)) return super.onKeyUp(keyCode, event)
    module.onKeyUpEvent(keyCode, event)
    return true
  }
`;

function patch(src) {
    if (src.includes(MARKER)) return src;

    let out = src;
    for (const imp of IMPORTS) {
        if (!out.includes(`import ${imp}`)) {
            out = out.replace(/^(package [^\n]+\n)/m, `$1\nimport ${imp}`);
        }
    }

    const classStart = /class MainActivity\s*:\s*ReactActivity\(\)\s*\{/;
    if (!classStart.test(out)) throw new Error("withVolumeKeys: could not find MainActivity class in MainActivity.kt");
    return out.replace(classStart, (m) => `${m}${OVERRIDES}`);
}

module.exports = function withVolumeKeys(config) {
    return withMainActivity(config, (config) => {
        if (config.modResults.language !== "kt") {
            throw new Error("withVolumeKeys: expected a Kotlin MainActivity");
        }
        config.modResults.contents = patch(config.modResults.contents);
        return config;
    });
};
