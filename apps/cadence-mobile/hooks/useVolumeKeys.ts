import { useEffect, useRef } from "react";
import { NativeModules, Platform } from "react-native";
import KeyEvent from "react-native-keyevent";

const KEYCODE_VOLUME_UP = 24;
const KEYCODE_VOLUME_DOWN = 25;

/**
 * Calls `onPress(+1)` / `onPress(-1)` for the phone's hardware volume up / down
 * buttons (including key repeats while held), while the app is in the foreground.
 * Android only. MainActivity forwards key events to react-native-keyevent and
 * consumes the volume keys; see plugins/withVolumeKeys.js.
 */
export function useVolumeKeys(onPress: (direction: 1 | -1) => void) {
    const latest = useRef(onPress);
    latest.current = onPress;

    useEffect(() => {
        if (Platform.OS !== "android") return;
        // Legacy modules load lazily under the New Architecture, and the library's JS
        // only listens on DeviceEventEmitter, so touch the module to create it.
        void NativeModules.KeyEventModule;

        KeyEvent.onKeyDownListener(({ keyCode }: { keyCode: number }) => {
            if (keyCode === KEYCODE_VOLUME_UP) latest.current(1);
            else if (keyCode === KEYCODE_VOLUME_DOWN) latest.current(-1);
        });
        return () => KeyEvent.removeKeyDownListener();
    }, []);
}
