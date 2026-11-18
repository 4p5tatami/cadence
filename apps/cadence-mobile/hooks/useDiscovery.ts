import { useCallback, useEffect, useRef, useState } from "react";
import Zeroconf, { type ZeroconfService } from "react-native-zeroconf";

export interface DiscoveredDevice {
    name: string;
    url: string;
}

const SERVICE = ["cadence", "tcp", "local."] as const;

export function useDiscovery() {
    const [devices, setDevices] = useState<DiscoveredDevice[]>([]);
    const zcRef = useRef<Zeroconf | null>(null);

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

        zc.scan(...SERVICE);

        return () => {
            zc.stop();
            zc.removeAllListeners();
            zcRef.current = null;
        };
    }, []);

    /** Clears the list and starts a fresh scan (scan() stops the previous one). */
    const rescan = useCallback(() => {
        setDevices([]);
        zcRef.current?.scan(...SERVICE);
    }, []);

    return { devices, rescan };
}
