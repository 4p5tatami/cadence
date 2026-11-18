import { useCallback, useEffect, useRef, useState } from "react";
import Zeroconf, { type ZeroconfService } from "react-native-zeroconf";

export interface DiscoveredDevice {
    name: string;
    url: string;
}

const SERVICE = ["cadence", "tcp", "local."] as const;
/** How long a manual rescan runs when auto-scan is off. */
const MANUAL_SCAN_MS = 8000;

/**
 * mDNS discovery of Cadence desktops.
 *
 * @param autoScan Scan continuously (e.g. while not connected). When false, the
 *   last results are kept and only `rescan()` searches, for MANUAL_SCAN_MS.
 */
export function useDiscovery(autoScan: boolean) {
    const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
    const [scanning, setScanning] = useState(false);
    const zcRef = useRef<Zeroconf | null>(null);
    const autoScanRef = useRef(autoScan);
    autoScanRef.current = autoScan;
    const stopTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

    // One Zeroconf instance for the lifetime of the hook.
    useEffect(() => {
        const zc = new Zeroconf();
        zcRef.current = zc;

        zc.on("resolved", (service: ZeroconfService) => {
            // Prefer IPv4: an IPv6 literal would need brackets in the URL and is often link-local.
            const ip = service.addresses?.find((a) => a.includes(".")) ?? service.addresses?.[0];
            if (!ip) return;
            const device = { name: service.name, url: `ws://${ip}:${service.port}` };
            setDevices((prev) => {
                const existing = prev.find((d) => d.name === device.name);
                if (existing?.url === device.url) return prev;
                // Replace in place so a device that changed IP keeps its position.
                return existing ? prev.map((d) => (d.name === device.name ? device : d)) : [...prev, device];
            });
        });

        zc.on("remove", (name: string) => {
            setDevices((prev) => prev.filter((d) => d.name !== name));
        });

        return () => {
            clearTimeout(stopTimer.current);
            zc.stop();
            zc.removeAllListeners();
            zcRef.current = null;
        };
    }, []);

    // Start or stop the continuous scan. Stopping keeps the current list.
    useEffect(() => {
        clearTimeout(stopTimer.current);
        if (autoScan) zcRef.current?.scan(...SERVICE);
        else zcRef.current?.stop();
        setScanning(autoScan);
    }, [autoScan]);

    /** Clears the list and searches again; time-limited when auto-scan is off. */
    const rescan = useCallback(() => {
        setDevices([]);
        setScanning(true);
        zcRef.current?.scan(...SERVICE); // scan() stops any running scan first
        clearTimeout(stopTimer.current);
        if (!autoScanRef.current) {
            stopTimer.current = setTimeout(() => {
                zcRef.current?.stop();
                setScanning(false);
            }, MANUAL_SCAN_MS);
        }
    }, []);

    return { devices, scanning, rescan };
}
